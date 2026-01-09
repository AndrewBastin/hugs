use std::fmt;
use std::path::Path;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

// ANSI color codes for styled error output
const BOLD_CYAN: &str = "\x1b[1;36m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// A path that displays with cyan highlighting
#[derive(Debug, Clone)]
pub struct StyledPath(pub String);

impl fmt::Display for StyledPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{BOLD_CYAN}{}{RESET}", self.0)
    }
}

impl From<&Path> for StyledPath {
    fn from(p: &Path) -> Self {
        StyledPath(p.display().to_string())
    }
}

impl From<String> for StyledPath {
    fn from(s: String) -> Self {
        StyledPath(s)
    }
}

impl From<&str> for StyledPath {
    fn from(s: &str) -> Self {
        StyledPath(s.to_string())
    }
}

impl From<std::path::PathBuf> for StyledPath {
    fn from(p: std::path::PathBuf) -> Self {
        StyledPath(p.display().to_string())
    }
}

impl From<&std::path::PathBuf> for StyledPath {
    fn from(p: &std::path::PathBuf) -> Self {
        StyledPath(p.display().to_string())
    }
}

/// A name/identifier that displays with yellow highlighting
#[derive(Debug, Clone)]
pub struct StyledName(pub String);

impl fmt::Display for StyledName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{YELLOW}{}{RESET}", self.0)
    }
}

impl From<String> for StyledName {
    fn from(s: String) -> Self {
        StyledName(s)
    }
}

impl From<&str> for StyledName {
    fn from(s: &str) -> Self {
        StyledName(s.to_string())
    }
}

/// A number that displays with bold highlighting
#[derive(Debug, Clone, Copy)]
pub struct StyledNum<T: fmt::Display>(pub T);

impl<T: fmt::Display> fmt::Display for StyledNum<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{BOLD}{}{RESET}", self.0)
    }
}

impl<T: fmt::Display> From<T> for StyledNum<T> {
    fn from(v: T) -> Self {
        StyledNum(v)
    }
}

/// The primary error type for all Hugs operations
#[derive(Error, Diagnostic, Debug)]
pub enum HugsError {
    // === Config Errors ===
    #[error("I couldn't parse your {path} file", path = StyledPath::from("config.toml"))]
    #[diagnostic(
        code(hugs::config::parse),
        help("I had trouble understanding your TOML syntax. Common issues include missing quotes around strings or unclosed brackets.")
    )]
    ConfigParse {
        #[source_code]
        src: NamedSource<String>,
        #[label("the error is around here")]
        span: SourceSpan,
        reason: String,
    },

    #[error("I couldn't read the config file at {path}")]
    #[diagnostic(
        code(hugs::config::read),
        help("Make sure the file exists and you have permission to read it.")
    )]
    ConfigRead {
        path: StyledPath,
        #[source]
        cause: std::io::Error,
    },

    // === Frontmatter Errors ===
    #[error("I couldn't parse the frontmatter in {file}")]
    #[diagnostic(
        code(hugs::frontmatter::parse),
        help("Make sure your frontmatter starts and ends with `---` and uses valid YAML syntax.\n\nExample:\n---\ntitle: My Page Title\ndescription: A short description\n---")
    )]
    FrontmatterParse {
        file: StyledPath,
        #[source_code]
        src: NamedSource<String>,
        #[label("{reason}")]
        span: SourceSpan,
        reason: String,
    },

    // === Template Errors ===
    #[error("I ran into a problem while rendering a template in {file}")]
    #[diagnostic(
        code(hugs::template::render),
        help("Check your Tera template syntax. Common issues include:\n- Unclosed {{ braces }} or {{% blocks %}}\n- Referencing a variable that doesn't exist\n- Incorrect filter usage")
    )]
    TemplateRender {
        file: StyledPath,
        #[source_code]
        src: NamedSource<String>,
        #[label("{reason}")]
        span: SourceSpan,
        reason: String,
    },

    #[error("I couldn't create the template context")]
    #[diagnostic(
        code(hugs::template::context),
        help("This is usually an internal error. The page data couldn't be serialized for the template.")
    )]
    TemplateContext { reason: String },

    // === File Errors ===
    #[error("I couldn't find a Hugs site at {path}")]
    #[diagnostic(
        code(hugs::site::not_found),
        help("Make sure the path points to a valid Hugs site directory. A Hugs site should contain:\n\n  <site>/\n    _/\n      header.md\n      footer.md\n      nav.md\n      theme.css\n    index.md\n    config.toml")
    )]
    SiteNotFound { path: StyledPath },

    #[error("I couldn't find the file at {path}")]
    #[diagnostic(
        code(hugs::file::not_found),
        help("Make sure the file exists and the path is correct.")
    )]
    FileNotFound { path: StyledPath },

    #[error("I couldn't read the file at {path}")]
    #[diagnostic(code(hugs::file::read))]
    FileRead {
        path: StyledPath,
        #[source]
        cause: std::io::Error,
    },

    #[error("I couldn't write to {path}")]
    #[diagnostic(code(hugs::file::write))]
    FileWrite {
        path: StyledPath,
        #[source]
        cause: std::io::Error,
    },

    #[error("I couldn't find your site's {styled_type} file", styled_type = StyledName::from(*file_type))]
    #[diagnostic(
        code(hugs::file::required_missing),
        help("{suggestion}")
    )]
    RequiredFileMissing {
        file_type: &'static str,
        expected_path: StyledPath,
        suggestion: String,
    },

    // === Feed Errors ===
    #[error("I need a title to generate the {feed_name} feed")]
    #[diagnostic(
        code(hugs::feed::missing_title),
        help("Add a title to your feed config or set `site.title` in config.toml:\n\n[site]\ntitle = \"My Site\"")
    )]
    FeedMissingTitle { feed_name: StyledName },

    #[error("I need a base URL to generate the {feed_name} feed")]
    #[diagnostic(
        code(hugs::feed::missing_url),
        help("Add `url` to the [site] section of your config.toml:\n\n[site]\nurl = \"https://example.com\"")
    )]
    FeedMissingUrl { feed_name: StyledName },

    // === Sitemap Errors ===
    #[error("I need a base URL to generate the {name}", name = StyledName::from("sitemap"))]
    #[diagnostic(
        code(hugs::sitemap::missing_url),
        help("Add `url` to the [site] section of your config.toml:\n\n[site]\nurl = \"https://example.com\"")
    )]
    SitemapMissingUrl,

    #[error("I ran into a problem generating the {name} template", name = StyledName::from("sitemap"))]
    #[diagnostic(code(hugs::sitemap::template))]
    SitemapTemplate { reason: String },

    // === Server Errors ===
    #[error("I couldn't start the server on port {port}")]
    #[diagnostic(code(hugs::server::port_bind))]
    PortBind {
        port: StyledNum<u16>,
        #[source_code]
        src: NamedSource<String>,
        #[label("this port is already in use")]
        span: SourceSpan,
        #[help]
        help_text: String,
        #[source]
        cause: std::io::Error,
    },

    #[error("I couldn't find an available port after trying ports {start_port} through {end_port}")]
    #[diagnostic(
        code(hugs::server::no_available_port),
        help("All ports in the range are in use. Try specifying a different starting port:\n\n    hugs dev <path> --port 9000")
    )]
    NoAvailablePort {
        start_port: StyledNum<u16>,
        end_port: StyledNum<u16>,
    },

    #[error("I couldn't start the file watcher")]
    #[diagnostic(
        code(hugs::watcher::init),
        help("This is usually a system-level issue. Make sure you have permission to watch the directory and that the system's file watcher limit hasn't been reached.")
    )]
    WatcherInit {
        #[source]
        cause: notify::Error,
    },

    #[error("I couldn't watch the directory at {path}")]
    #[diagnostic(code(hugs::watcher::path))]
    WatcherPath {
        path: StyledPath,
        #[source]
        cause: notify::Error,
    },

    // === Path Errors ===
    #[error("I found an unexpected path structure while processing {path}")]
    #[diagnostic(
        code(hugs::path::strip_prefix),
        help("I expected the path to be inside {base}, but it wasn't. This might indicate a bug in the site structure.")
    )]
    PathStripPrefix { path: StyledPath, base: StyledPath },

    #[error("I found a file path with characters I can't handle: {path}")]
    #[diagnostic(
        code(hugs::path::invalid_utf8),
        help("File and directory names should use UTF-8 characters. Try renaming the file to use standard characters.")
    )]
    PathInvalidUtf8 { path: StyledPath },

    // === Markdown Errors ===
    #[error("I couldn't parse the markdown in {file}")]
    #[diagnostic(
        code(hugs::markdown::parse),
        help("There was a problem converting your markdown to HTML. Check for any unusual syntax.")
    )]
    MarkdownParse { file: StyledPath, reason: String },

    // === Dynamic Page Errors ===
    #[error("Dynamic page {file} is missing parameter values for `{param_name}`")]
    #[diagnostic(
        code(hugs::dynamic::missing_param),
        help("The filename uses [{param_name}] but the frontmatter doesn't define values for it.\n\nAdd to your frontmatter:\n{param_name}: [value1, value2, value3]")
    )]
    DynamicMissingParam {
        file: StyledPath,
        param_name: StyledName,
    },

    #[error("I couldn't parse the dynamic parameter config in {file}")]
    #[diagnostic(
        code(hugs::dynamic::param_parse),
        help("The `{param_name}` field must be either an array or a Tera expression string.\n\nExamples:\n{param_name}: [1, 2, 3]\n{param_name}: \"{{{{ range(end=5) }}}}\"")
    )]
    DynamicParamParse {
        file: StyledPath,
        param_name: StyledName,
        reason: String,
    },

    #[error("I couldn't evaluate the Tera expression for `{param_name}` in {file}")]
    #[diagnostic(
        code(hugs::dynamic::expr_eval),
        help("The expression `{expression}` failed to evaluate.\n\nMake sure it produces an array. Common functions:\n- range(end=5) -> [0, 1, 2, 3, 4]\n- range(start=1, end=6) -> [1, 2, 3, 4, 5]")
    )]
    DynamicExprEval {
        file: StyledPath,
        param_name: StyledName,
        expression: String,
        reason: String,
    },

    // === Build Errors ===
    #[error("I couldn't resolve the page at URL {url}")]
    #[diagnostic(
        code(hugs::build::resolve_page),
        help("I was looking for a markdown file at {file_path} but couldn't find or process it.")
    )]
    PageResolve { url: StyledPath, file_path: StyledPath },

    #[error("A background task failed: {reason}")]
    #[diagnostic(
        code(hugs::build::task_join),
        help("A parallel task panicked or was cancelled during the build.")
    )]
    TaskJoin { reason: String },

    // === New Site Errors ===
    #[error("I can't create a site at {path} because the directory is not empty")]
    #[diagnostic(
        code(hugs::new::dir_not_empty),
        help("Choose an empty directory or a path that doesn't exist yet.")
    )]
    DirNotEmpty { path: StyledPath },

    #[error("I couldn't create the output directory at {path}")]
    #[diagnostic(code(hugs::build::create_dir))]
    CreateDir {
        path: StyledPath,
        #[source]
        cause: std::io::Error,
    },

    #[error("I couldn't copy the file from {src} to {dest}")]
    #[diagnostic(code(hugs::build::copy_file))]
    CopyFile {
        src: StyledPath,
        dest: StyledPath,
        #[source]
        cause: std::io::Error,
    },

    // === Server Runtime Errors ===
    #[error("The server encountered an error")]
    #[diagnostic(code(hugs::server::runtime))]
    ServerRuntime {
        #[source]
        cause: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, HugsError>;

impl HugsError {
    /// Create a config parse error with source span from a TOML error
    pub fn config_parse(path: &Path, content: &str, error: toml::de::Error) -> Self {
        let span = error
            .span()
            .map(|r| SourceSpan::new(r.start.into(), (r.end - r.start).max(1).into()))
            .unwrap_or_else(|| SourceSpan::from((0_usize, 1_usize)));

        HugsError::ConfigParse {
            src: NamedSource::new(path.display().to_string(), content.to_string()),
            span,
            reason: error.message().to_string(),
        }
    }

    /// Create a template render error, attempting to extract line info from Tera error
    pub fn template_render(path: &Path, content: &str, error: tera::Error) -> Self {
        // Use Debug format which includes more details like line numbers
        let error_str = format!("{:?}", error);
        let span = extract_tera_span(&error_str, content);
        // Use Display for the user-facing reason (cleaner)
        let reason = error.to_string();

        HugsError::TemplateRender {
            file: StyledPath::from(path),
            src: NamedSource::new(path.display().to_string(), content.to_string()),
            span,
            reason,
        }
    }

    /// Create a template render error with a custom path name (for inline templates)
    pub fn template_render_named(name: &str, content: &str, error: &tera::Error) -> Self {
        // Use Debug format which includes more details like line numbers
        let error_str = format!("{:?}", error);
        let span = extract_tera_span(&error_str, content);
        // Use Display for the user-facing reason (cleaner)
        let reason = error.to_string();

        HugsError::TemplateRender {
            file: StyledPath::from(name),
            src: NamedSource::new(name.to_string(), content.to_string()),
            span,
            reason,
        }
    }

    /// Create a port bind error with command source and highlighted port
    pub fn port_bind(path: &Path, port: u16, cause: std::io::Error) -> Self {
        use owo_colors::OwoColorize;

        let command = format!("hugs dev {} --port {}", path.display(), port);
        let port_str = port.to_string();
        let port_start = command.rfind(&port_str).unwrap_or(0);
        let span = SourceSpan::new(port_start.into(), port_str.len().into());

        let alt_port = port.checked_add(1).unwrap_or(8081);
        let help_text = format!(
            "Port {} is already in use. You can either:\n\n  \
            1. Try a different port: {}\n\n  \
            2. Omit {} to let me find an available port automatically",
            port.bold(),
            format!("hugs dev <path> --port {}", alt_port).cyan(),
            "--port".cyan().bold()
        );

        HugsError::PortBind {
            port: port.into(),
            src: NamedSource::new("command".to_string(), command),
            span,
            help_text,
            cause,
        }
    }
}

/// Extract source span from Tera error message by looking for line numbers
fn extract_tera_span(error_str: &str, content: &str) -> SourceSpan {
    // Tera errors often contain patterns like "--> 13:1" or "at line 5"
    if let Some(line_num) = extract_line_number(error_str) {
        let offset: usize = content
            .lines()
            .take(line_num.saturating_sub(1))
            .map(|l| l.len() + 1)
            .sum();

        let line_len = content
            .lines()
            .nth(line_num.saturating_sub(1))
            .map(|l| l.len().max(1))
            .unwrap_or(1);

        SourceSpan::new(offset.into(), line_len.into())
    } else {
        SourceSpan::from((0_usize, 1_usize))
    }
}

/// Try to extract a line number from an error string
fn extract_line_number(s: &str) -> Option<usize> {
    // Try "--> LINE:COL" pattern (pest/Tera parser errors)
    // e.g., "--> 13:1" means line 13, column 1
    if let Some(idx) = s.find("--> ") {
        let rest = &s[idx + 4..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num_str.parse() {
            return Some(n);
        }
    }

    // Try "at line X" pattern
    if let Some(idx) = s.find("at line ") {
        let rest = &s[idx + 8..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num_str.parse() {
            return Some(n);
        }
    }

    // Try "__tera_one_off:X" pattern
    if let Some(idx) = s.find("__tera_one_off:") {
        let rest = &s[idx + 15..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num_str.parse() {
            return Some(n);
        }
    }

    // Try "template:X" pattern
    if let Some(idx) = s.find("template:") {
        let rest = &s[idx + 9..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num_str.parse() {
            return Some(n);
        }
    }

    None
}

/// Extension trait for adding Hugs error context to IO operations
pub trait HugsResultExt<T> {
    /// Add file read context to an error
    fn with_file_read(self, path: &Path) -> Result<T>;
}

impl<T> HugsResultExt<T> for std::result::Result<T, std::io::Error> {
    fn with_file_read(self, path: &Path) -> Result<T> {
        self.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                HugsError::FileNotFound {
                    path: StyledPath::from(path),
                }
            } else {
                HugsError::FileRead {
                    path: StyledPath::from(path),
                    cause: e,
                }
            }
        })
    }
}

/// Render a HugsError as HTML for in-browser display during development
pub fn render_error_html(error: &HugsError) -> String {
    use std::fmt::Write;

    let mut html = String::new();

    // Use miette's debug output which includes the fancy formatting
    let error_text = format!("{:?}", miette::Report::new_boxed(Box::new(error.clone())));

    // Convert ANSI escape codes to styled HTML spans
    let escaped = ansi_to_html::convert(&error_text)
        .unwrap_or_else(|_| {
            // Fallback: escape HTML manually if conversion fails
            error_text
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        })
        .replace('\n', "<br>");

    write!(
        html,
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Error - Hugs</title>
    <style>
        body {{
            font-family: 'SF Mono', 'Menlo', 'Monaco', 'Consolas', monospace;
            background-color: #1a1a2e;
            color: #eee;
            padding: 2rem;
            margin: 0;
            line-height: 1.6;
        }}
        .error-container {{
            max-width: 900px;
            margin: 0 auto;
            background: #16213e;
            border-radius: 8px;
            padding: 2rem;
            border-left: 4px solid #e94560;
        }}
        .error-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 1rem;
        }}
        .error-face {{
            font-size: 1.5rem;
            color: #e94560;
        }}
        .error-title {{
            color: #e94560;
            font-size: 1.2rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .error-content {{
            white-space: pre-wrap;
            font-size: 0.9rem;
            overflow-x: auto;
        }}
        .help-text {{
            margin-top: 1.5rem;
            padding: 1rem;
            background: #0f3460;
            border-radius: 4px;
            border-left: 3px solid #00d9ff;
        }}
    </style>
</head>
<body>
    <div class="error-container">
        <div class="error-header">
            <div class="error-title">
                <span>✕</span>
                <span>Something went wrong</span>
            </div>
            <div class="error-face">(╥﹏╥)</div>
        </div>
        <div class="error-content">{}</div>
    </div>
</body>
</html>"#,
        escaped
    )
    .unwrap();

    html
}

// Implement Clone for HugsError where possible (needed for render_error_html)
impl Clone for HugsError {
    fn clone(&self) -> Self {
        match self {
            HugsError::ConfigParse { src, span, reason } => HugsError::ConfigParse {
                src: NamedSource::new(src.name().to_string(), src.inner().clone()),
                span: *span,
                reason: reason.clone(),
            },
            HugsError::ConfigRead { path, cause } => HugsError::ConfigRead {
                path: path.clone(),
                cause: std::io::Error::new(cause.kind(), cause.to_string()),
            },
            HugsError::FrontmatterParse { file, src, span, reason } => HugsError::FrontmatterParse {
                file: file.clone(),
                src: NamedSource::new(src.name().to_string(), src.inner().clone()),
                span: *span,
                reason: reason.clone(),
            },
            HugsError::TemplateRender { file, src, span, reason } => HugsError::TemplateRender {
                file: file.clone(),
                src: NamedSource::new(src.name().to_string(), src.inner().clone()),
                span: *span,
                reason: reason.clone(),
            },
            HugsError::TemplateContext { reason } => {
                HugsError::TemplateContext { reason: reason.clone() }
            }
            HugsError::SiteNotFound { path } => HugsError::SiteNotFound { path: path.clone() },
            HugsError::FileNotFound { path } => HugsError::FileNotFound { path: path.clone() },
            HugsError::FileRead { path, cause } => HugsError::FileRead {
                path: path.clone(),
                cause: std::io::Error::new(cause.kind(), cause.to_string()),
            },
            HugsError::FileWrite { path, cause } => HugsError::FileWrite {
                path: path.clone(),
                cause: std::io::Error::new(cause.kind(), cause.to_string()),
            },
            HugsError::RequiredFileMissing { file_type, expected_path, suggestion } => {
                HugsError::RequiredFileMissing {
                    file_type,
                    expected_path: expected_path.clone(),
                    suggestion: suggestion.clone(),
                }
            }
            HugsError::FeedMissingTitle { feed_name } => {
                HugsError::FeedMissingTitle { feed_name: feed_name.clone() }
            }
            HugsError::FeedMissingUrl { feed_name } => {
                HugsError::FeedMissingUrl { feed_name: feed_name.clone() }
            }
            HugsError::SitemapMissingUrl => HugsError::SitemapMissingUrl,
            HugsError::SitemapTemplate { reason } => {
                HugsError::SitemapTemplate { reason: reason.clone() }
            }
            HugsError::PortBind { port, src, span, help_text, cause } => HugsError::PortBind {
                port: StyledNum(port.0),
                src: NamedSource::new(src.name().to_string(), src.inner().clone()),
                span: *span,
                help_text: help_text.clone(),
                cause: std::io::Error::new(cause.kind(), cause.to_string()),
            },
            HugsError::NoAvailablePort { start_port, end_port } => HugsError::NoAvailablePort {
                start_port: StyledNum(start_port.0),
                end_port: StyledNum(end_port.0),
            },
            HugsError::WatcherInit { cause } => HugsError::WatcherInit {
                cause: notify::Error::generic(&cause.to_string()),
            },
            HugsError::WatcherPath { path, cause } => HugsError::WatcherPath {
                path: path.clone(),
                cause: notify::Error::generic(&cause.to_string()),
            },
            HugsError::PathStripPrefix { path, base } => HugsError::PathStripPrefix {
                path: path.clone(),
                base: base.clone(),
            },
            HugsError::PathInvalidUtf8 { path } => {
                HugsError::PathInvalidUtf8 { path: path.clone() }
            }
            HugsError::MarkdownParse { file, reason } => HugsError::MarkdownParse {
                file: file.clone(),
                reason: reason.clone(),
            },
            HugsError::DynamicMissingParam { file, param_name } => HugsError::DynamicMissingParam {
                file: file.clone(),
                param_name: param_name.clone(),
            },
            HugsError::DynamicParamParse { file, param_name, reason } => HugsError::DynamicParamParse {
                file: file.clone(),
                param_name: param_name.clone(),
                reason: reason.clone(),
            },
            HugsError::DynamicExprEval { file, param_name, expression, reason } => HugsError::DynamicExprEval {
                file: file.clone(),
                param_name: param_name.clone(),
                expression: expression.clone(),
                reason: reason.clone(),
            },
            HugsError::PageResolve { url, file_path } => HugsError::PageResolve {
                url: url.clone(),
                file_path: file_path.clone(),
            },
            HugsError::TaskJoin { reason } => HugsError::TaskJoin {
                reason: reason.clone(),
            },
            HugsError::DirNotEmpty { path } => HugsError::DirNotEmpty {
                path: path.clone(),
            },
            HugsError::CreateDir { path, cause } => HugsError::CreateDir {
                path: path.clone(),
                cause: std::io::Error::new(cause.kind(), cause.to_string()),
            },
            HugsError::CopyFile { src, dest, cause } => HugsError::CopyFile {
                src: src.clone(),
                dest: dest.clone(),
                cause: std::io::Error::new(cause.kind(), cause.to_string()),
            },
            HugsError::ServerRuntime { cause } => HugsError::ServerRuntime {
                cause: std::io::Error::new(cause.kind(), cause.to_string()),
            },
        }
    }
}
