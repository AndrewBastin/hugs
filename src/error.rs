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
        help("{help_text}")
    )]
    TemplateRender {
        file: StyledPath,
        #[source_code]
        src: NamedSource<String>,
        #[label("{reason}")]
        span: SourceSpan,
        reason: String,
        help_text: String,
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

    #[error("I couldn't find a Hugs site in the current directory")]
    #[diagnostic(
        code(hugs::site::not_found_cwd),
        help("Make sure you're in a Hugs site directory, or specify a path:\n\n    hugs dev <path>\n    hugs build <path>\n\nA Hugs site should contain:\n\n  <site>/\n    _/\n      header.md\n      footer.md\n      nav.md\n      theme.css\n    index.md\n    config.toml")
    )]
    SiteNotFoundCwd,

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
        help("The `{param_name}` field must be either an array or a Jinja expression string.\n\nExamples:\n{param_name}: [1, 2, 3]\n{param_name}: \"{{{{ range(end=5) }}}}\"")
    )]
    DynamicParamParse {
        file: StyledPath,
        param_name: StyledName,
        reason: String,
    },

    #[error("I couldn't evaluate the Jinja expression for `{param_name}` in {file}")]
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

    // === Macro Errors ===
    #[error("I couldn't parse the macro in {file}")]
    #[diagnostic(
        code(hugs::macros::parse),
        help("Make sure your macro file has valid frontmatter with parameter definitions.\n\nExample:\n---\ntitle: \"\"\nvariant: \"default\"\n---\n<div>Macro content here</div>")
    )]
    MacroParse {
        file: StyledPath,
        reason: String,
    },

    #[error("The macro filename '{name}' in {path} is not a valid identifier")]
    #[diagnostic(
        code(hugs::macros::invalid_name),
        help("Macro names must start with a letter or underscore and contain only letters, numbers, and underscores.\n\nExamples of valid names: card, my_button, Button2")
    )]
    MacroInvalidName {
        path: StyledPath,
        name: StyledName,
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

    #[error("I couldn't read your input: {cause}")]
    #[diagnostic(code(hugs::new::input_error))]
    InputError { cause: String },

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

    // === Doc Command Errors ===
    #[error("I couldn't create a temporary directory for the documentation")]
    #[diagnostic(
        code(hugs::doc::temp_dir),
        help("Make sure you have write permissions to the system temp directory.")
    )]
    DocTempDir {
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

    /// Create a template render error, attempting to extract line info from MiniJinja error
    pub fn template_render(
        path: &Path,
        content: &str,
        error: minijinja::Error,
        hints: &TemplateHints,
        macro_prefix_bytes: usize,
        macro_prefix_lines: usize,
    ) -> Self {
        let span = extract_template_span(&error, content, macro_prefix_bytes, macro_prefix_lines);
        let reason = format_template_error_reason(&error);
        let help_text = template_error_help(&error, hints);

        HugsError::TemplateRender {
            file: StyledPath::from(path),
            src: NamedSource::new(path.display().to_string(), content.to_string()),
            span,
            reason,
            help_text,
        }
    }

    /// Create a template render error with a custom path name (for inline templates)
    pub fn template_render_named(
        name: &str,
        content: &str,
        error: &minijinja::Error,
        hints: &TemplateHints,
        macro_prefix_bytes: usize,
        macro_prefix_lines: usize,
    ) -> Self {
        let span = extract_template_span(error, content, macro_prefix_bytes, macro_prefix_lines);
        let reason = format_template_error_reason(error);
        let help_text = template_error_help(error, hints);

        HugsError::TemplateRender {
            file: StyledPath::from(name),
            src: NamedSource::new(name.to_string(), content.to_string()),
            span,
            reason,
            help_text,
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

/// Extract source span from MiniJinja error, adjusting for macro prefix
/// Uses byte range if available (debug feature), otherwise falls back to line number
fn extract_template_span(
    error: &minijinja::Error,
    content: &str,
    macro_prefix_bytes: usize,
    macro_prefix_lines: usize,
) -> SourceSpan {
    // Try byte range first (most precise) - requires debug feature
    if let Some(range) = error.range() {
        // Adjust for macro prefix
        let adjusted_start = range.start.saturating_sub(macro_prefix_bytes);
        let adjusted_end = range.end.saturating_sub(macro_prefix_bytes);

        // If error is in macro prefix, point to start of user content
        if adjusted_start == 0 && range.start < macro_prefix_bytes {
            return SourceSpan::from((0_usize, 1_usize));
        }

        // Clamp range to content bounds
        let start = adjusted_start.min(content.len());
        let end = adjusted_end.min(content.len());
        let len = (end - start).max(1);
        return SourceSpan::new(start.into(), len.into());
    }

    // Fall back to line number
    if let Some(line_num) = error.line() {
        // Adjust line number for macro prefix
        let adjusted_line = line_num.saturating_sub(macro_prefix_lines);

        // If error is in macro prefix, point to start of user content
        if adjusted_line == 0 && line_num <= macro_prefix_lines {
            return SourceSpan::from((0_usize, 1_usize));
        }

        let offset: usize = content
            .lines()
            .take(adjusted_line.saturating_sub(1))
            .map(|l| l.len() + 1)
            .sum();

        let line_len = content
            .lines()
            .nth(adjusted_line.saturating_sub(1))
            .map(|l| l.len().max(1))
            .unwrap_or(1);

        return SourceSpan::new(offset.into(), line_len.into());
    }

    SourceSpan::from((0_usize, 1_usize))
}

/// Format a clean error message from MiniJinja error
/// Uses detail() for cleaner messages when available
fn format_template_error_reason(error: &minijinja::Error) -> String {
    // detail() provides a cleaner message without the full context
    if let Some(detail) = error.detail() {
        return detail.to_string();
    }
    error.to_string()
}

/// Template hints extracted from the MiniJinja environment for error suggestions
#[derive(Clone, Default)]
pub struct TemplateHints {
    pub filters: Vec<String>,
    pub functions: Vec<String>,
    pub tests: Vec<String>,
    pub variables: Vec<String>,
    pub macros: Vec<String>,
}

impl TemplateHints {
    /// Extract hints from a MiniJinja environment
    ///
    /// For functions: uses `env.globals()` to get all registered globals (including functions)
    /// For filters/tests: uses documented MiniJinja builtins (no introspection API available)
    /// For variables: uses the known PageContent struct fields
    pub fn from_environment(env: &minijinja::Environment) -> Self {
        // Extract functions dynamically from globals
        let functions: Vec<String> = env
            .globals()
            .map(|(name, _)| name.to_string())
            .collect();

        // MiniJinja builtin filters (from minijinja 2.x documentation)
        // https://docs.rs/minijinja/latest/minijinja/filters/
        let filters = vec![
            // Type conversion
            "bool", "float", "int", "list", "string",
            // String operations
            "capitalize", "escape", "e", "lower", "replace", "safe", "split", "title", "trim", "upper", "urlencode",
            // Sequence operations
            "batch", "chain", "first", "join", "last", "length", "lines", "reverse", "slice", "sort", "unique", "zip",
            // Numeric operations
            "abs", "max", "min", "round", "sum",
            // Object/Dictionary operations
            "attr", "dictsort", "items",
            // Filtering/Selection
            "default", "d", "map", "reject", "rejectattr", "select", "selectattr",
            // Grouping
            "groupby",
            // Output formatting
            "format", "indent", "pprint", "tojson",
        ].into_iter().map(String::from).collect();

        // MiniJinja builtin tests (from minijinja 2.x documentation)
        // https://docs.rs/minijinja/latest/minijinja/tests/
        // Note: tests are used without the "is_" prefix in templates (e.g., `is odd` not `is is_odd`)
        let tests = vec![
            "boolean", "defined", "divisibleby", "endingwith", "eq", "equalto",
            "even", "false", "filter", "float", "ge", "gt", "in", "integer",
            "iterable", "le", "lower", "lt", "mapping", "ne", "none", "number",
            "odd", "safe", "sameas", "sequence", "startingwith", "string",
            "test", "true", "undefined", "upper",
        ].into_iter().map(String::from).collect();

        // Variables from PageContent struct (our code, so we know these)
        let variables = vec![
            "title", "content", "url", "base", "path_class",
            "header", "nav", "footer", "dev_script", "seo",
            "syntax_highlighting_enabled",
        ].into_iter().map(String::from).collect();

        Self { filters, functions, tests, variables, macros: Vec::new() }
    }

    /// Set the available macro names (for error suggestions)
    pub fn with_macros(mut self, macros: Vec<String>) -> Self {
        self.macros = macros;
        self
    }
}

/// Calculate edit distance between two strings (Levenshtein distance)
fn edit_distance(a: &str, b: &str) -> usize {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 { return n; }
    if n == 0 { return m; }

    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// Find the best fuzzy match from a list of candidates
fn find_best_match<'a>(name: &str, candidates: &'a [String]) -> Option<&'a str> {
    let name_lower = name.to_lowercase();
    let max_distance = (name.len() / 2).max(2);

    candidates
        .iter()
        .filter_map(|candidate| {
            let distance = edit_distance(&name_lower, candidate);
            if distance <= max_distance && distance > 0 {
                Some((candidate.as_str(), distance))
            } else {
                None
            }
        })
        .min_by_key(|(_, distance)| *distance)
        .map(|(candidate, _)| candidate)
}

/// Extract the problematic identifier from an error detail
fn extract_identifier(detail: &str) -> Option<&str> {
    // Match patterns like: `foo`, 'foo', "foo"
    if let Some(start) = detail.find('`') {
        let rest = &detail[start + 1..];
        if let Some(end) = rest.find('`') {
            return Some(&rest[..end]);
        }
    }
    if let Some(start) = detail.find('\'') {
        let rest = &detail[start + 1..];
        if let Some(end) = rest.find('\'') {
            return Some(&rest[..end]);
        }
    }

    // MiniJinja format: "filter NAME is unknown", "function NAME is unknown", etc.
    // Also handles: "variable NAME is undefined", "test NAME is unknown"
    let patterns = [
        "filter ", "function ", "variable ", "test ", "method ",
    ];
    for pattern in patterns {
        if let Some(start) = detail.find(pattern) {
            let rest = &detail[start + pattern.len()..];
            // Find the end of the identifier (space or end of string)
            let end = rest.find(' ').unwrap_or(rest.len());
            if end > 0 {
                return Some(&rest[..end]);
            }
        }
    }

    // MiniJinja also uses: "NAME is unknown" or "NAME is undefined" (without type prefix)
    if let Some(pos) = detail.find(" is unknown") {
        let before = &detail[..pos];
        // Get the last word before " is unknown"
        if let Some(start) = before.rfind(' ') {
            return Some(&before[start + 1..]);
        } else if !before.is_empty() {
            return Some(before);
        }
    }
    if let Some(pos) = detail.find(" is undefined") {
        let before = &detail[..pos];
        if let Some(start) = before.rfind(' ') {
            return Some(&before[start + 1..]);
        } else if !before.is_empty() {
            return Some(before);
        }
    }

    None
}

/// Generate contextual help text based on the error kind
fn template_error_help(error: &minijinja::Error, hints: &TemplateHints) -> String {
    use minijinja::ErrorKind;

    let detail = error.detail().unwrap_or_default();
    let identifier = extract_identifier(detail);

    match error.kind() {
        ErrorKind::UndefinedError => {
            let mut help = String::from(
                "I couldn't find this variable or attribute in the template context.\n\n"
            );

            if let Some(name) = identifier {
                if let Some(suggestion) = find_best_match(name, &hints.variables) {
                    help.push_str(&format!(
                        "Hint: Did you mean `{}`?\n\n",
                        suggestion
                    ));
                }
            }

            help.push_str(
                "Make sure it's spelled correctly and defined in your page frontmatter.\n\
                 Common variables: title, content, url, path"
            );
            help
        }
        ErrorKind::UnknownFilter => {
            let mut help = String::from(
                "I don't recognize this filter.\n\n"
            );

            if let Some(name) = identifier {
                if let Some(suggestion) = find_best_match(name, &hints.filters) {
                    help.push_str(&format!(
                        "Hint: Did you mean `{}`?\n\n",
                        suggestion
                    ));
                }
            }

            help.push_str(
                "Here are some filters you can use:\n\
                 - Text: safe, escape, lower, upper, title, trim, replace\n\
                 - Lists: first, last, length, reverse, sort, join\n\
                 - Values: default, int, float, abs, round"
            );
            help
        }
        ErrorKind::UnknownFunction => {
            let mut help = String::from(
                "I don't recognize this function or macro.\n\n"
            );

            if let Some(name) = identifier {
                // Check both functions and macros for suggestions
                let func_suggestion = find_best_match(name, &hints.functions);
                let macro_suggestion = find_best_match(name, &hints.macros);

                match (func_suggestion, macro_suggestion) {
                    (Some(f), Some(m)) => {
                        help.push_str(&format!(
                            "Hint: Did you mean the function `{}` or the macro `{}`?\n\n",
                            f, m
                        ));
                    }
                    (Some(f), None) => {
                        help.push_str(&format!(
                            "Hint: Did you mean `{}`?\n\n",
                            f
                        ));
                    }
                    (None, Some(m)) => {
                        help.push_str(&format!(
                            "Hint: Did you mean the macro `{}`?\n\n",
                            m
                        ));
                    }
                    (None, None) => {}
                }
            }

            // Build a dynamic list of available functions
            let func_list = if hints.functions.is_empty() {
                "No custom functions available".to_string()
            } else {
                hints.functions.join(", ")
            };

            help.push_str(&format!(
                "Available functions: {}\n\n\
                 Common usage:\n\
                 - pages(within='/blog/') - get a list of pages\n\
                 - cache_bust(path='/file.css') - add cache-busting hash\n\
                 - range(end=5) - generate a sequence of numbers",
                func_list
            ));

            // Show available macros if any exist
            if !hints.macros.is_empty() {
                help.push_str(&format!(
                    "\n\nAvailable macros: {}\n\
                     Use macros with: {{% call macro_name() %}}...{{% endcall %}}",
                    hints.macros.join(", ")
                ));
            }

            help
        }
        ErrorKind::UnknownTest => {
            let mut help = String::from(
                "I don't recognize this test.\n\n"
            );

            if let Some(name) = identifier {
                if let Some(suggestion) = find_best_match(name, &hints.tests) {
                    help.push_str(&format!(
                        "Hint: Did you mean `{}`?\n\n",
                        suggestion
                    ));
                }
            }

            help.push_str(
                "Here are some tests you can use:\n\
                 - Existence: defined, undefined, none\n\
                 - Numbers: odd, even, divisibleby(n)\n\
                 - Comparison: eq, ne, lt, le, gt, ge"
            );
            help
        }
        ErrorKind::SyntaxError => {
            "I had trouble parsing this template.\n\n\
             Here are some things to check:\n\
             - Are all your {{ braces }} and {% blocks %} properly closed?\n\
             - Do you have matching {% endif %}, {% endfor %}, etc.?\n\
             - Are strings properly quoted?"
                .to_string()
        }
        ErrorKind::MissingArgument => {
            "It looks like this function is missing a required argument.\n\n\
             Double-check the function signature and make sure you've provided \
             all the required parameters."
                .to_string()
        }
        ErrorKind::TooManyArguments => {
            "This function received more arguments than it expects.\n\n\
             Double-check the function signature - you may have an extra parameter."
                .to_string()
        }
        ErrorKind::InvalidOperation => {
            "I can't perform this operation on these types of values.\n\n\
             For example, you can't add a string to a number, or access \
             an attribute on something that isn't an object."
                .to_string()
        }
        ErrorKind::CannotUnpack => {
            "I couldn't unpack this value.\n\n\
             Make sure you're iterating over something that's actually a list, \
             or that the value can be destructured the way you're trying to."
                .to_string()
        }
        _ => {
            "I ran into a problem with this template.\n\n\
             Here are some things to check:\n\
             - Are all your {{ braces }} and {% blocks %} properly closed?\n\
             - Are you referencing variables that exist?\n\
             - Are filters and functions spelled correctly?"
                .to_string()
        }
    }
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
pub fn render_error_html(error: &HugsError, dev_script: &str) -> String {
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
    {}
</body>
</html>"#,
        escaped,
        dev_script
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
            HugsError::TemplateRender { file, src, span, reason, help_text } => HugsError::TemplateRender {
                file: file.clone(),
                src: NamedSource::new(src.name().to_string(), src.inner().clone()),
                span: *span,
                reason: reason.clone(),
                help_text: help_text.clone(),
            },
            HugsError::TemplateContext { reason } => {
                HugsError::TemplateContext { reason: reason.clone() }
            }
            HugsError::SiteNotFound { path } => HugsError::SiteNotFound { path: path.clone() },
            HugsError::SiteNotFoundCwd => HugsError::SiteNotFoundCwd,
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
            HugsError::MacroParse { file, reason } => HugsError::MacroParse {
                file: file.clone(),
                reason: reason.clone(),
            },
            HugsError::MacroInvalidName { path, name } => HugsError::MacroInvalidName {
                path: path.clone(),
                name: name.clone(),
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
            HugsError::InputError { cause } => HugsError::InputError {
                cause: cause.clone(),
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
            HugsError::DocTempDir { cause } => HugsError::DocTempDir {
                cause: std::io::Error::new(cause.kind(), cause.to_string()),
            },
        }
    }
}
