use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use actix_web::{HttpResponse, http::header::ContentType};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml::Value as YamlValue;
use sha2::{Sha256, Digest};
use chrono::{DateTime, Locale, NaiveDate, NaiveDateTime, Utc};
use minijinja::{Environment, State, Value};
use tokio::task::JoinSet;
use walkdir::WalkDir;

use crate::config::SiteConfig;
use crate::console;
use crate::error::{HugsError, HugsResultExt, Result, TemplateHints};

/// Create markdown options (can't be static due to non-Send callback fields)
fn markdown_options() -> markdown::Options {
    markdown::Options {
        parse: markdown::ParseOptions::gfm(),
        compile: markdown::CompileOptions {
            allow_any_img_src: true,
            allow_dangerous_html: true,
            allow_dangerous_protocol: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Convert markdown to HTML with optional syntax highlighting for code blocks
fn markdown_to_html(
    body: &str,
    config: &crate::config::SyntaxHighlightConfig,
) -> std::result::Result<String, String> {
    let html = markdown::to_html_with_options(body, &markdown_options())
        .map_err(|e| e.to_string())?;

    if config.enabled {
        Ok(crate::highlight::highlight_code_blocks(&html, &config.theme))
    } else {
        Ok(html)
    }
}

/// Create a `pages` function for minijinja that returns all pages, optionally filtered by URL prefix
fn create_pages_function(
    pages: Arc<Vec<PageInfo>>,
) -> impl Fn(minijinja::value::Kwargs) -> std::result::Result<Value, minijinja::Error> + Send + Sync + 'static {
    move |kwargs: minijinja::value::Kwargs| {
        // If `within` arg is provided, filter by URL prefix
        let within: Option<String> = kwargs.get("within")?;
        if let Some(prefix) = within {
            // The index URL for the directory is the prefix with a trailing slash
            let index_url = if prefix.ends_with('/') {
                prefix.clone()
            } else {
                format!("{}/", prefix)
            };
            let filtered: Vec<&PageInfo> = pages
                .iter()
                .filter(|page| {
                    // Include pages within the prefix, but exclude the directory index
                    page.url.starts_with(&prefix) && page.url != index_url
                })
                .collect();

            Ok(Value::from_serialize(&filtered))
        } else {
            Ok(Value::from_serialize(&*pages))
        }
    }
}

/// Registry tracking which files need cache-busted copies.
/// Maps original path (e.g., "/theme.css") to hashed path (e.g., "/theme.a1b2c3f4.css")
#[derive(Default, Clone)]
pub struct CacheBustRegistry {
    entries: Arc<Mutex<HashMap<String, String>>>,
}

impl CacheBustRegistry {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn entries(&self) -> HashMap<String, String> {
        self.entries.lock().unwrap().clone()
    }

    fn insert(&self, original: &str, hashed: &str) {
        self.entries
            .lock()
            .unwrap()
            .insert(original.to_string(), hashed.to_string());
    }
}

/// Data for cache busting function - used to create the minijinja function
/// Usage in templates: {{ cache_bust(path="/theme.css") }} -> "/theme.a1b2c3f4.css"
#[derive(Clone)]
pub struct CacheBustFunction {
    site_path: PathBuf,
    theme_css: String,
    highlight_css: String,
    registry: CacheBustRegistry,
}

impl CacheBustFunction {
    pub fn new(
        site_path: PathBuf,
        theme_css: String,
        highlight_css: String,
        registry: CacheBustRegistry,
    ) -> Self {
        Self {
            site_path,
            theme_css,
            highlight_css,
            registry,
        }
    }

    /// Create a minijinja-compatible function from this cache bust configuration
    pub fn to_minijinja_fn(&self) -> impl Fn(minijinja::value::Kwargs) -> std::result::Result<String, minijinja::Error> + Send + Sync + 'static {
        let site_path = self.site_path.clone();
        let theme_css = self.theme_css.clone();
        let highlight_css = self.highlight_css.clone();
        let registry = self.registry.clone();

        move |kwargs: minijinja::value::Kwargs| {
            let path: Option<String> = kwargs.get("path")?;
            let path = path.ok_or_else(|| {
                minijinja::Error::new(
                    minijinja::ErrorKind::MissingArgument,
                    "cache_bust requires 'path' argument",
                )
            })?;
            // Check if already computed
            {
                let entries = registry.entries.lock().unwrap();
                if let Some(hashed) = entries.get(&path) {
                    return Ok(hashed.clone());
                }
            }

            // Get content (special case for theme.css and highlight.css which are pre-loaded)
            let content = if path == "/theme.css" {
                theme_css.as_bytes().to_vec()
            } else if path == "/highlight.css" {
                highlight_css.as_bytes().to_vec()
            } else {
                let file_path = if path.starts_with('/') {
                    site_path.join(&path[1..])
                } else {
                    site_path.join(&path)
                };
                std::fs::read(&file_path).map_err(|e| {
                    minijinja::Error::new(
                        minijinja::ErrorKind::InvalidOperation,
                        format!("cache_bust: cannot read file '{}': {}", path, e),
                    )
                })?
            };

            // Compute hash (first 8 hex chars of SHA-256)
            let hash = compute_content_hash(&content);
            let hashed_path = insert_hash_into_path(&path, &hash);

            // Register for build phase
            registry.insert(&path, &hashed_path);

            Ok(hashed_path)
        }
    }
}

/// Compute SHA-256 hash and return first 8 hex characters
fn compute_content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    hex::encode(&result[..4]) // 4 bytes = 8 hex chars
}

/// Insert hash into path before extension: /theme.css -> /theme.a1b2c3f4.css
fn insert_hash_into_path(path: &str, hash: &str) -> String {
    if let Some(dot_pos) = path.rfind('.') {
        format!("{}.{}{}", &path[..dot_pos], hash, &path[dot_pos..])
    } else {
        format!("{}.{}", path, hash)
    }
}

pub const ROOT_TEMPL: &'static str = include_str!("templates/root.jinja");

/// Error type that includes both the MiniJinja error and template hints for suggestions
pub struct TemplateError {
    pub error: minijinja::Error,
    pub hints: TemplateHints,
    /// Number of bytes in the macro prefix (for adjusting byte ranges)
    pub macro_prefix_bytes: usize,
    /// Number of lines in the macro prefix (for adjusting line numbers)
    pub macro_prefix_lines: usize,
}

/// Help marker prefixes used to identify help requests in error messages
pub const HELP_MARKER_FUNCTION: &str = "__hugs_help_function__";
pub const HELP_MARKER_FILTER: &str = "__hugs_help_filter__";
pub const HELP_MARKER_TEST: &str = "__hugs_help_test__";

/// MiniJinja builtin filters (from minijinja 2.x documentation)
/// https://docs.rs/minijinja/latest/minijinja/filters/
const BUILTIN_FILTERS: &[&str] = &[
    // Type conversion
    "bool", "float", "int", "list", "string",
    // String operations
    "capitalize", "escape", "e", "lower", "replace", "safe", "split", "title", "trim", "upper", "urlencode",
    // Sequence operations
    "batch", "chain", "first", "flatten", "join", "last", "length", "lines", "reverse", "slice", "sort", "unique", "zip",
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
    // Hugs custom filters
    "datefmt", "help",
];

/// MiniJinja builtin tests (from minijinja 2.x documentation)
/// https://docs.rs/minijinja/latest/minijinja/tests/
const BUILTIN_TESTS: &[&str] = &[
    "boolean", "defined", "divisibleby", "endingwith", "eq", "equalto",
    "even", "false", "filter", "float", "ge", "gt", "in", "integer",
    "iterable", "le", "lower", "lt", "mapping", "ne", "none", "number",
    "odd", "safe", "sameas", "sequence", "startingwith", "string",
    "test", "true", "undefined", "upper", "help",
];

/// Wrap a list of items into lines with a max width
fn wrap_items_to_lines(items: &[&str], max_width: usize) -> String {
    let mut result = String::new();
    let mut current_line = String::from("  ");

    for (i, item) in items.iter().enumerate() {
        let separator = if i == 0 { "" } else { ", " };
        let with_sep = format!("{}{}", separator, item);

        if current_line.len() + with_sep.len() > max_width && current_line.len() > 2 {
            result.push_str(&current_line);
            result.push('\n');
            current_line = format!("  {}", item);
        } else {
            current_line.push_str(&with_sep);
        }
    }

    if !current_line.trim().is_empty() {
        result.push_str(&current_line);
        result.push('\n');
    }

    result
}

/// Create the `help` function for minijinja
/// Usage: {{ help() }} - shows all available variables, functions, filters, tests, macros
fn create_help_function(
    function_names: Vec<String>,
) -> impl Fn(&State) -> std::result::Result<Value, minijinja::Error> + Send + Sync + 'static {
    move |state: &State| {
        use base64::{Engine, engine::general_purpose::STANDARD};
        
        // Collect variables with their values, sorted alphabetically
        let mut names: Vec<String> = state
            .known_variables()
            .into_iter()
            .map(|c| c.into_owned())
            .collect();
        names.sort();
        
        let var_entries: Vec<String> = names
            .into_iter()
            .filter_map(|name| {
                // Filter out registered functions (they appear in known_variables but aren't variables)
                if function_names.contains(&name) {
                    return None;
                }
                let value_repr = state
                    .lookup(&name)
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_else(|| "?".to_string());
                // Encode name and value as base64 to handle all special characters
                let name_b64 = STANDARD.encode(&name);
                let value_b64 = STANDARD.encode(&value_repr);
                Some(format!("{}:{}", name_b64, value_b64))
            })
            .collect();

        let msg = format!(
            "{}:variables={}",
            HELP_MARKER_FUNCTION,
            var_entries.join(",")
        );

        Err(minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            msg,
        ))
    }
}

/// Create the `help` filter for minijinja
/// Usage: {{ value | help }} - shows the value's type/content and applicable filters
fn create_help_filter() -> impl Fn(&State, Value) -> std::result::Result<Value, minijinja::Error> + Send + Sync + 'static {
    use base64::{Engine, engine::general_purpose::STANDARD};
    
    |_state: &State, value: Value| {
        let value_kind = format!("{:?}", value.kind());
        let value_repr = format!("{:?}", value);
        let value_b64 = STANDARD.encode(&value_repr);

        let msg = format!(
            "{}:kind={}:value={}",
            HELP_MARKER_FILTER,
            value_kind,
            value_b64
        );

        Err(minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            msg,
        ))
    }
}

/// Create the `help` test for minijinja
/// Usage: {% if value is help %} - shows the value's type/content and applicable tests
fn create_help_test() -> impl Fn(&State, Value) -> std::result::Result<bool, minijinja::Error> + Send + Sync + 'static {
    use base64::{Engine, engine::general_purpose::STANDARD};
    
    |_state: &State, value: Value| {
        let value_kind = format!("{:?}", value.kind());
        let value_repr = format!("{:?}", value);
        let value_b64 = STANDARD.encode(&value_repr);

        let msg = format!(
            "{}:kind={}:value={}",
            HELP_MARKER_TEST,
            value_kind,
            value_b64
        );

        Err(minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            msg,
        ))
    }
}

/// Create the `readtime` function for minijinja
/// Usage: {{ readtime(text) }} - returns estimated reading time in minutes for the given markdown text
fn create_readtime_function(
    reading_speed: u32,
) -> impl Fn(String) -> std::result::Result<u32, minijinja::Error> + Send + Sync + 'static {
    move |text: String| {
        let word_count = count_words_in_markdown(&text);
        let minutes = (word_count as f64 / reading_speed as f64).ceil() as u32;
        Ok(minutes.max(1))
    }
}

/// Parse a locale string into a chrono Locale.
/// Normalizes hyphens to underscores (e.g., "en-US" -> "en_US").
fn parse_locale(s: &str) -> Option<Locale> {
    let normalized = s.replace('-', "_");
    Locale::try_from(normalized.as_str()).ok()
}

/// Parse a date string into a DateTime<Utc>.
/// Supports: ISO 8601/RFC 3339, YYYY-MM-DD, YYYY-MM-DD HH:MM:SS
fn parse_date_string_for_filter(s: &str) -> std::result::Result<DateTime<Utc>, minijinja::Error> {
    // ISO 8601 / RFC 3339 (2024-01-15T10:30:00Z)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // YYYY-MM-DD (2024-01-15)
    if let Ok(nd) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        if let Some(ndt) = nd.and_hms_opt(0, 0, 0) {
            return Ok(DateTime::from_naive_utc_and_offset(ndt, Utc));
        }
    }

    // YYYY-MM-DD HH:MM:SS (2024-01-15 10:30:00)
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(DateTime::from_naive_utc_and_offset(ndt, Utc));
    }

    Err(minijinja::Error::new(
        minijinja::ErrorKind::InvalidOperation,
        format!(
            "datefmt: couldn't parse date '{}'. Supported formats: YYYY-MM-DD, YYYY-MM-DDTHH:MM:SSZ, YYYY-MM-DD HH:MM:SS",
            s
        ),
    ))
}

/// Create the `datefmt` filter for locale-aware date formatting.
///
/// Usage in templates:
///   {{ page.date | datefmt("%B %d, %Y") }}
///   {{ page.date | datefmt("%A, %d %B %Y", locale="fr_FR") }}
fn create_datefmt_filter(
    default_locale: String,
) -> impl Fn(&State, Value, String, minijinja::value::Kwargs) -> std::result::Result<String, minijinja::Error>
       + Send
       + Sync
       + 'static {
    // Pre-parse the default locale at filter creation time
    let default_locale_parsed = parse_locale(&default_locale).unwrap_or(Locale::POSIX);

    move |_state: &State, value: Value, format: String, kwargs: minijinja::value::Kwargs| {
        // Get the locale from kwargs or use default
        let locale_str: Option<String> = kwargs.get("locale")?;
        kwargs.assert_all_used()?;

        let locale = match locale_str {
            Some(ref s) => parse_locale(s).unwrap_or(default_locale_parsed),
            None => default_locale_parsed,
        };

        // Parse the date value
        let datetime = match value.as_str() {
            Some(s) => parse_date_string_for_filter(s)?,
            None => {
                return Err(minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    format!("datefmt: expected a date string, got {}", value.kind()),
                ))
            }
        };

        // Format with locale
        Ok(datetime.format_localized(&format, locale).to_string())
    }
}

/// Create the `flatten` filter for flattening nested sequences.
///
/// Usage in templates:
///   {{ nested_list | flatten }}
///   {{ [[1, 2], [3, 4]] | flatten }}  -> [1, 2, 3, 4]
fn create_flatten_filter(
) -> impl Fn(&State, Value) -> std::result::Result<Value, minijinja::Error> + Send + Sync + 'static
{
    |_state: &State, value: Value| {
        let iter = value.try_iter().map_err(|_| {
            minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                format!("flatten: expected a sequence, got {}", value.kind()),
            )
        })?;

        let mut result = Vec::new();
        for item in iter {
            match item.try_iter() {
                Ok(inner_iter) => {
                    for inner_item in inner_iter {
                        result.push(inner_item);
                    }
                }
                Err(_) => {
                    result.push(item);
                }
            }
        }

        Ok(Value::from_iter(result))
    }
}

/// Count words in markdown content, stripping HTML tags and markdown syntax
fn count_words_in_markdown(text: &str) -> usize {
    let without_code_blocks = strip_code_blocks(text);
    let without_html = strip_html_tags(&without_code_blocks);
    let without_markdown = strip_markdown_syntax(&without_html);
    without_markdown.split_whitespace().count()
}

fn strip_code_blocks(text: &str) -> String {
    let mut result = String::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }
        if !in_code_block {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

fn strip_html_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

fn strip_markdown_syntax(text: &str) -> String {
    let mut result = text.to_string();
    result = result.replace("**", "");
    result = result.replace("__", "");
    result = result.replace("~~", "");
    result = result.replace('*', "");
    result = result.replace('_', " ");
    result = result.replace('#', "");
    result = result.replace('>', "");
    result = result.replace('[', "");
    result = result.replace(']', "");
    result = result.replace('(', " ");
    result = result.replace(')', " ");
    result = result.replace('`', "");
    result
}

/// Create a configured template environment with custom functions
fn create_template_env(
    pages: &Arc<Vec<PageInfo>>,
    cache_bust: Option<&CacheBustFunction>,
    reading_speed: u32,
    default_language: &str,
) -> (Environment<'static>, TemplateHints) {
    let mut env = Environment::new();
    env.add_function("pages", create_pages_function(Arc::clone(pages)));
    env.add_function("readtime", create_readtime_function(reading_speed));
    if let Some(cb) = cache_bust {
        env.add_function("cache_bust", cb.to_minijinja_fn());
    }

    // Add the datefmt filter with the site's default locale
    env.add_filter("datefmt", create_datefmt_filter(default_language.to_string()));

    // Add the flatten filter for flattening nested sequences
    env.add_filter("flatten", create_flatten_filter());

    // Collect function names before adding help (includes builtins + our functions)
    let mut function_names: Vec<String> = env.globals().map(|(name, _)| name.to_string()).collect();
    function_names.push("help".to_string()); // include help itself
    env.add_function("help", create_help_function(function_names));
    env.add_filter("help", create_help_filter());
    env.add_test("help", create_help_test());

    let hints = TemplateHints::from_environment(&env);
    (env, hints)
}

/// Extract macro names from a macros template string
/// Looks for patterns like `{% macro NAME(...) %}`
fn extract_macro_names(macros_template: &str) -> Vec<String> {
    let mut names = Vec::new();
    // Simple regex-free parsing: look for "{% macro " followed by identifier
    for line in macros_template.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("{%") {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix("macro") {
                let rest = rest.trim();
                // Extract the macro name (identifier before '(')
                let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
                if name_end > 0 {
                    names.push(rest[..name_end].to_string());
                }
            }
        }
    }
    names
}

pub fn render_template<T: serde::Serialize>(
    template: &str,
    ctx: T,
    pages: &Arc<Vec<PageInfo>>,
    cache_bust: Option<&CacheBustFunction>,
    macros_template: &str,
    reading_speed: u32,
    default_language: &str,
) -> std::result::Result<String, TemplateError> {
    let (mut env, hints) = create_template_env(pages, cache_bust, reading_speed, default_language);

    // Extract macro names and add them to hints for error suggestions
    let macro_names = extract_macro_names(macros_template);
    let hints = hints.with_macros(macro_names);

    // Calculate macro prefix metrics for error position adjustment
    let (macro_prefix_bytes, macro_prefix_lines) = if !macros_template.is_empty() {
        // +1 for the joining newline
        (macros_template.len() + 1, macros_template.lines().count() + 1)
    } else {
        (0, 0)
    };

    // Prepend macro definitions directly to template so they're globally available
    let full_template = if !macros_template.is_empty() {
        format!("{}\n{}", macros_template, template)
    } else {
        template.to_string()
    };

    let make_err = |e| TemplateError { error: e, hints: hints.clone(), macro_prefix_bytes, macro_prefix_lines };
    env.add_template("template", &full_template).map_err(make_err)?;
    let tmpl = env.get_template("template").map_err(make_err)?;
    tmpl.render(ctx).map_err(|e| TemplateError { error: e, hints, macro_prefix_bytes, macro_prefix_lines })
}

/// Render using the root template
pub fn render_root_template<T: serde::Serialize>(
    app_data: &AppData,
    ctx: T,
    cache_bust: &CacheBustFunction,
) -> std::result::Result<String, TemplateError> {
    let (mut env, hints) = create_template_env(&app_data.pages, Some(cache_bust), app_data.config.build.reading_speed, &app_data.config.site.language);

    // Extract macro names and add them to hints for error suggestions
    let macro_names = extract_macro_names(&app_data.macros_template);
    let hints = hints.with_macros(macro_names);

    // Calculate macro prefix metrics for error position adjustment
    let (macro_prefix_bytes, macro_prefix_lines) = if !app_data.macros_template.is_empty() {
        // +1 for the joining newline
        (app_data.macros_template.len() + 1, app_data.macros_template.lines().count() + 1)
    } else {
        (0, 0)
    };

    // Prepend macro definitions to root template so they're globally available
    let full_root_template = if !app_data.macros_template.is_empty() {
        format!("{}\n{}", app_data.macros_template, ROOT_TEMPL)
    } else {
        ROOT_TEMPL.to_string()
    };

    let make_err = |e| TemplateError { error: e, hints: hints.clone(), macro_prefix_bytes, macro_prefix_lines };
    env.add_template("root", &full_root_template).map_err(make_err)?;
    let tmpl = env.get_template("root").map_err(make_err)?;
    tmpl.render(ctx).map_err(|e| TemplateError { error: e, hints, macro_prefix_bytes, macro_prefix_lines })
}

fn parse_md(
    content_jinja_md: &str,
    page_content: &PageContent<'_>,
    pages: &Arc<Vec<PageInfo>>,
    source_name: &str,
    macros_template: &str,
    reading_speed: u32,
    default_language: &str,
) -> Result<String> {
    let content_md = render_template(content_jinja_md, page_content, pages, None, macros_template, reading_speed, default_language)
        .map_err(|e| HugsError::template_render_named(
            source_name,
            content_jinja_md,
            &e.error,
            &e.hints,
            e.macro_prefix_bytes,
            e.macro_prefix_lines,
        ))?;

    markdown::to_html_with_options(&content_md, &markdown_options()).map_err(|e| HugsError::MarkdownParse {
        file: source_name.into(),
        reason: e.to_string(),
    })
}

#[derive(Clone)]
pub struct AppData {
    pub site_path: PathBuf,

    pub header_html: String,
    pub footer_html: String,
    pub nav_html: String,

    pub theme_css: String,

    /// All pages including expanded dynamic pages
    pub pages: Arc<Vec<PageInfo>>,

    /// Dynamic page definitions (for dev server pattern matching)
    pub dynamic_defs: Arc<Vec<DynamicPageDef>>,

    pub notfound_page: Option<PathBuf>,

    pub config: SiteConfig,

    pub cache_bust_registry: CacheBustRegistry,

    /// Pre-generated CSS for syntax highlighting
    pub highlight_css: String,

    /// Pre-built template containing all macro definitions from _/macros/
    pub macros_template: String,

    /// Content template from _/content.md (defaults to "{{ content }}")
    pub content_template: String,
}

impl AppData {
    /// Create a CacheBustFunction configured for this site
    pub fn cache_bust_function(&self) -> CacheBustFunction {
        CacheBustFunction::new(
            self.site_path.clone(),
            self.theme_css.clone(),
            self.highlight_css.clone(),
            self.cache_bust_registry.clone(),
        )
    }
}

async fn read_required_file(
    path: &Path,
    file_type: &'static str,
    relative_path: &str,
) -> Result<String> {
    tokio::fs::read_to_string(path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HugsError::RequiredFileMissing {
                file_type,
                expected_path: relative_path.into(),
                suggestion: format!(
                    "I was looking for `{}`. This file provides the {} content that appears on every page. Create it to continue.",
                    relative_path, file_type
                ),
            }
        } else {
            HugsError::FileRead {
                path: path.into(),
                cause: e,
            }
        }
    })
}

impl AppData {
    pub async fn load(site_path: PathBuf, command: &str) -> Result<AppData> {
        // Check if this looks like a valid Hugs site
        let underscore_dir = site_path.join("_");
        if !site_path.is_dir() || !underscore_dir.is_dir() {
            // Use a friendlier error message when running in the current directory
            if site_path.as_os_str() == "." {
                return Err(HugsError::site_not_found_cwd(command));
            }
            return Err(HugsError::site_not_found(&site_path));
        }

        let header_path = site_path.join("_/header.md");
        let footer_path = site_path.join("_/footer.md");
        let nav_path = site_path.join("_/nav.md");
        let theme_path = site_path.join("_/theme.css");
        let content_template_path = site_path.join("_/content.md");

        let header_md = read_required_file(&header_path, "header", "_/header.md").await?;
        let footer_md = read_required_file(&footer_path, "footer", "_/footer.md").await?;
        let nav_md = read_required_file(&nav_path, "navigation", "_/nav.md").await?;
        let theme_css = read_required_file(&theme_path, "theme stylesheet", "_/theme.css").await?;
        let content_template = if content_template_path.exists() {
            tokio::fs::read_to_string(&content_template_path).await.map_err(|e| HugsError::FileRead {
                path: content_template_path.clone().into(),
                cause: e,
            })?
        } else {
            String::from("{{ content }}")
        };
        let config = SiteConfig::load(&site_path).await?;

        // Initialize syntax highlighting registry and generate CSS
        crate::highlight::init_registry();
        let highlight_css = if config.build.syntax_highlighting.enabled {
            crate::highlight::generate_theme_css(&config.build.syntax_highlighting.theme)
        } else {
            String::new()
        };

        // Load macros from _/macros/ directory
        let macros = load_macros(&site_path).await?;
        let macros_template = build_macros_template(&macros);

        // Phase 1: Scan pages and collect static pages + raw dynamic definitions
        let raw_scan_result = scan_pages_raw(&site_path).await?;

        // Create initial pages Arc with just static pages (for dynamic param evaluation)
        let static_pages = Arc::new(raw_scan_result.static_pages.clone());

        // Phase 2: Evaluate dynamic page parameters (now pages() is available)
        let dynamic_defs = evaluate_dynamic_defs(raw_scan_result.raw_dynamic_defs, &static_pages)?;

        // Expand dynamic pages into concrete pages
        let expanded_pages = expand_dynamic_pages(&dynamic_defs);

        // Combine static and expanded pages
        let mut all_pages = raw_scan_result.static_pages;
        all_pages.extend(expanded_pages);

        let pages = Arc::new(all_pages);
        let dynamic_defs = Arc::new(dynamic_defs);

        let initial_page_content = PageContent {
            title: "",
            header: "",
            footer: "",
            nav: "",
            content: "",
            main_content: "",
            path_class: "",
            base: "/",
            dev_script: "",
            seo: SeoContext::default(),
            syntax_highlighting_enabled: false,
        };

        let reading_speed = config.build.reading_speed;
        let default_language = &config.site.language;
        let header_html = parse_md(&header_md, &initial_page_content, &pages, "_/header.md", &macros_template, reading_speed, default_language)?;
        let footer_html = parse_md(&footer_md, &initial_page_content, &pages, "_/footer.md", &macros_template, reading_speed, default_language)?;
        let nav_html = parse_md(&nav_md, &initial_page_content, &pages, "_/nav.md", &macros_template, reading_speed, default_language)?;

        let notfound_path = site_path.join("[404].md");
        let notfound_page = if notfound_path.exists() {
            Some(notfound_path)
        } else {
            None
        };

        Ok(AppData {
            site_path,
            header_html,
            footer_html,
            nav_html,
            theme_css,
            pages,
            dynamic_defs,
            notfound_page,
            config,
            cache_bust_registry: CacheBustRegistry::new(),
            highlight_css,
            macros_template,
            content_template,
        })
    }
}

#[derive(Deserialize)]
pub struct ContentFrontmatter {
    pub title: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub image: Option<String>,
}

#[derive(Serialize, Default, Clone)]
pub struct SeoContext {
    pub description: Option<String>,
    pub author: Option<String>,
    pub canonical_url: String,
    pub og_title: String,
    pub og_description: Option<String>,
    pub og_url: String,
    pub og_type: String,
    pub og_image: Option<String>,
    pub og_site_name: Option<String>,
    pub twitter_card: String,
    pub twitter_title: String,
    pub twitter_description: Option<String>,
    pub twitter_image: Option<String>,
    pub twitter_handle: Option<String>,
}

/// Render a page title using the site's title template, if configured.
/// Returns the original title if no template is set or if rendering fails.
fn render_title_template(
    page_title: &str,
    site: &crate::config::SiteMetadata,
) -> String {
    match &site.title_template {
        Some(template) => {
            let mut env = Environment::new();
            if let Err(_) = env.add_template("title", template) {
                return page_title.to_string();
            }

            let tmpl = match env.get_template("title") {
                Ok(t) => t,
                Err(_) => return page_title.to_string(),
            };

            let ctx = minijinja::context! {
                title => page_title,
                site => minijinja::context! {
                    title => site.title.as_deref().unwrap_or("")
                }
            };

            tmpl.render(ctx).unwrap_or_else(|_| page_title.to_string())
        }
        None => page_title.to_string(),
    }
}

pub fn build_seo_context(
    frontmatter: &ContentFrontmatter,
    page_url: &str,
    site: &crate::config::SiteMetadata,
) -> SeoContext {
    let base_url = site.url.as_deref().unwrap_or("").trim_end_matches('/');
    let page_url_clean = page_url.trim_end_matches('/');
    let canonical_url = if page_url_clean.is_empty() {
        format!("{}/", base_url)
    } else {
        format!("{}{}", base_url, page_url_clean)
    };

    let description = frontmatter.description.clone().or_else(|| site.description.clone());
    let author = frontmatter.author.clone().or_else(|| site.author.clone());

    let image = frontmatter
        .image
        .as_ref()
        .or(site.default_image.as_ref())
        .map(|img| {
            if img.starts_with("http") {
                img.clone()
            } else {
                format!("{}{}", base_url.trim_end_matches('/'), img)
            }
        });

    let twitter_card = if image.is_some() {
        "summary_large_image".to_string()
    } else {
        "summary".to_string()
    };

    let rendered_title = render_title_template(&frontmatter.title, site);

    SeoContext {
        description: description.clone(),
        author,
        canonical_url: canonical_url.clone(),
        og_title: rendered_title.clone(),
        og_description: description.clone(),
        og_url: canonical_url,
        og_type: "website".to_string(),
        og_image: image.clone(),
        og_site_name: site.title.clone(),
        twitter_card,
        twitter_title: rendered_title,
        twitter_description: description,
        twitter_image: image,
        twitter_handle: site.twitter_handle.clone(),
    }
}

#[derive(Clone, Serialize)]
pub struct PageInfo {
    pub url: String,
    pub file_path: String,
    #[serde(flatten)]
    pub frontmatter: YamlValue,
}

/// Dynamic page template before expansion (e.g., `[slug].md`)
#[derive(Clone)]
pub struct DynamicPageDef {
    /// The parameter name extracted from filename (e.g., "slug" from "[slug].md")
    pub param_name: String,
    /// The source file path relative to site root (e.g., "blog/[slug].md")
    pub source_path: PathBuf,
    /// The evaluated parameter values
    pub param_values: Vec<YamlValue>,
    /// The raw frontmatter for this dynamic page
    pub frontmatter: YamlValue,
}

/// Raw dynamic page definition before parameter evaluation
/// Used in two-phase scanning where we first collect all pages, then evaluate dynamic params
#[derive(Clone)]
struct RawDynamicPageDef {
    param_name: String,
    source_path: PathBuf,
    frontmatter: YamlValue,
    /// Full file content for error reporting with source spans
    file_content: String,
}

/// A parsed macro definition from _/macros/*.md
#[derive(Clone, Debug)]
pub struct MacroDefinition {
    /// The macro name (derived from filename, e.g., "card" from "card.md")
    pub name: String,
    /// Parameter definitions with default values (from frontmatter)
    pub params: Vec<MacroParam>,
    /// The raw body content (markdown/HTML/Jinja template)
    pub body: String,
    /// Source file path for error reporting (kept for future use)
    #[allow(dead_code)]
    pub source_path: PathBuf,
}

/// A single macro parameter with its default value
#[derive(Clone, Debug)]
pub struct MacroParam {
    pub name: String,
    /// Minijinja-compatible default value literal (e.g., "", "default", none, 123)
    pub default_value: String,
}

/// Result of scanning pages - separates static pages from raw dynamic definitions
/// Dynamic parameter values are not yet evaluated (requires pages to be available)
struct RawScanResult {
    static_pages: Vec<PageInfo>,
    raw_dynamic_defs: Vec<RawDynamicPageDef>,
}

/// Context for rendering a dynamic page - contains the parameter name and value
#[derive(Clone)]
pub struct DynamicContext {
    pub param_name: String,
    pub param_value: YamlValue,
}

impl DynamicContext {
    /// Create a DynamicContext from a PageInfo if it's a dynamic page
    pub fn from_page_info(page_info: &PageInfo) -> Option<Self> {
        // Check if the file_path contains a dynamic parameter pattern
        let file_path = &page_info.file_path;
        if !file_path.contains('[') || !file_path.contains(']') {
            return None;
        }

        // Extract param name from file path (e.g., "blog/[slug].md" -> "slug")
        let filename = std::path::Path::new(file_path).file_name()?.to_str()?;
        let param_name = extract_param_name(filename)?;

        // Get param value from frontmatter
        let param_value = page_info.frontmatter.get(&param_name)?.clone();

        Some(DynamicContext {
            param_name,
            param_value,
        })
    }

    /// Get the parameter name and JSON value for this dynamic context
    pub fn to_json_pair(&self) -> (String, serde_json::Value) {
        let json_value = yaml_to_json_value(&self.param_value);
        (self.param_name.clone(), json_value)
    }
}

/// Convert a YAML value to a JSON value
fn yaml_to_json_value(value: &YamlValue) -> serde_json::Value {
    match value {
        YamlValue::Null => serde_json::Value::Null,
        YamlValue::Bool(b) => serde_json::Value::Bool(*b),
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::json!(f)
            } else {
                serde_json::Value::String(n.to_string())
            }
        }
        YamlValue::String(s) => serde_json::Value::String(s.clone()),
        YamlValue::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_to_json_value).collect())
        }
        YamlValue::Mapping(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        YamlValue::String(s) => s.clone(),
                        _ => return None,
                    };
                    Some((key, yaml_to_json_value(v)))
                })
                .collect();
            serde_json::Value::Object(obj)
        }
        YamlValue::Tagged(tagged) => yaml_to_json_value(&tagged.value),
    }
}

/// Check if a file path represents a dynamic page (e.g., `[slug].md`)
fn is_dynamic_page(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.starts_with('[') && name.ends_with("].md"))
        .unwrap_or(false)
}

/// Extract parameter name from a dynamic page filename
/// e.g., "[slug].md" -> Some("slug")
fn extract_param_name(filename: &str) -> Option<String> {
    filename
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix("].md"))
        .map(String::from)
}

/// Evaluate parameter values from frontmatter with access to pages() and other helpers.
/// This is the enhanced version that provides helper functions in the evaluation context.
fn evaluate_param_values_with_pages(
    param_name: &str,
    frontmatter: &YamlValue,
    source_path: &Path,
    pages: &Arc<Vec<PageInfo>>,
    file_content: &str,
) -> Result<Vec<YamlValue>> {
    use miette::{NamedSource, SourceSpan};

    // Helper to find the span of the param expression in the file content
    let find_param_span = |expr: &str| -> SourceSpan {
        // Look for the pattern "param_name: " or "param_name:" followed by the expression
        let search_patterns = [
            format!("{}: \"{}\"", param_name, expr),
            format!("{}:\"{}\"", param_name, expr),
            format!("{}: '{}'", param_name, expr),
            format!("{}:'{}'", param_name, expr),
        ];

        for pattern in &search_patterns {
            if let Some(pos) = file_content.find(pattern) {
                // Point to the expression part (after "param_name: ")
                let expr_start = pos + param_name.len() + 2; // +2 for ": " or ":"
                return SourceSpan::new(expr_start.into(), (pattern.len() - param_name.len() - 2).into());
            }
        }

        // Fallback: try to find just the expression string
        if let Some(pos) = file_content.find(expr) {
            return SourceSpan::new(pos.into(), expr.len().into());
        }

        // Last resort: point to start of file
        SourceSpan::new(0_usize.into(), 1_usize.into())
    };

    // Helper to parse help filter/test marker and extract kind/value
    let parse_help_marker = |detail: &str, marker: &str| -> Option<(String, String)> {
        use base64::{Engine, engine::general_purpose::STANDARD};

        let rest = detail.strip_prefix(marker)?;
        let mut kind = String::new();
        let mut value_b64 = String::new();

        for part in rest.split(':') {
            if let Some(k) = part.strip_prefix("kind=") {
                kind = k.to_string();
            } else if let Some(v) = part.strip_prefix("value=") {
                value_b64 = v.to_string();
            }
        }

        let value = STANDARD
            .decode(&value_b64)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_else(|| "?".to_string());

        Some((kind, value))
    };

    // Helper to create the error with all fields
    let make_error = |expr: &str, reason: String, resolved_value: Option<String>| -> HugsError {
        let span = find_param_span(expr);

        // Check if this is a help request - if so, provide specialized help
        // Use the same span labels as template errors
        let (display_reason, help_text, resolved) = if reason.starts_with(HELP_MARKER_FILTER) {
            // Filter help: "you asked for filter help here"
            if let Some((kind, value)) = parse_help_marker(&reason, HELP_MARKER_FILTER) {
                use owo_colors::OwoColorize;
                let friendly_reason = "you asked for filter help here".to_string();
                let filters_list = wrap_items_to_lines(BUILTIN_FILTERS, 60);
                let help = format!(
                    "You're filtering a {} with value:\n    {}\n\n\
                     Filters you can apply:\n{}\n\
                     I'm trying to determine the routes for this dynamic page.\n\
                     Make sure it produces an array of values.",
                    kind.yellow().bold(),
                    value.bright_yellow(),
                    filters_list
                );
                (friendly_reason, help, Some(value))
            } else {
                // Fallback if parsing fails
                let help = format!(
                    "The expression `{}` failed to evaluate.{}\n\nI'm trying to determine the routes for this dynamic page.\nMake sure it produces an array of values.\n\nCommon functions:\n- range(end=5) -> [0, 1, 2, 3, 4]\n- range(start=1, end=6) -> [1, 2, 3, 4, 5]\n- pages(within='/blog') | map(attribute='slug') | list",
                    expr,
                    resolved_value.as_ref().map(|v| format!("\n\nThe expression resolved to:\n{}", v)).unwrap_or_default()
                );
                (reason, help, resolved_value)
            }
        } else if reason.starts_with(HELP_MARKER_TEST) {
            // Test help: "you asked for test help here"
            if let Some((kind, value)) = parse_help_marker(&reason, HELP_MARKER_TEST) {
                use owo_colors::OwoColorize;
                let friendly_reason = "you asked for test help here".to_string();
                let tests_list = wrap_items_to_lines(BUILTIN_TESTS, 60);
                let help = format!(
                    "You're testing a {} with value:\n    {}\n\n\
                     Tests you can use:\n{}\n\
                     I'm trying to determine the routes for this dynamic page.\n\
                     Make sure it produces an array of values.",
                    kind.yellow().bold(),
                    value.bright_yellow(),
                    tests_list
                );
                (friendly_reason, help, Some(value))
            } else {
                let help = format!(
                    "The expression `{}` failed to evaluate.{}\n\nI'm trying to determine the routes for this dynamic page.\nMake sure it produces an array of values.\n\nCommon functions:\n- range(end=5) -> [0, 1, 2, 3, 4]\n- range(start=1, end=6) -> [1, 2, 3, 4, 5]\n- pages(within='/blog') | map(attribute='slug') | list",
                    expr,
                    resolved_value.as_ref().map(|v| format!("\n\nThe expression resolved to:\n{}", v)).unwrap_or_default()
                );
                (reason, help, resolved_value)
            }
        } else if reason.starts_with(HELP_MARKER_FUNCTION) {
            // Function help: "you asked for help here"
            let friendly_reason = "you asked for help here".to_string();
            let filters_list = wrap_items_to_lines(BUILTIN_FILTERS, 60);
            let tests_list = wrap_items_to_lines(BUILTIN_TESTS, 60);
            let help = format!(
                "Variables you can use:\n\
                 In dynamic page expressions, no variables are pre-defined.\n\
                 Use pages() to get page data.\n\n\
                 Functions you can call:\n\
                 pages(), help()\n\n\
                 Filters you can apply:\n{}\n\
                 Tests you can use:\n{}\n\
                 I'm trying to determine the routes for this dynamic page.\n\
                 Make sure it produces an array of values.",
                filters_list,
                tests_list
            );
            (friendly_reason, help, None)
        } else {
            let help = format!(
                "The expression `{}` failed to evaluate.{}\n\nI'm trying to determine the routes for this dynamic page.\nMake sure it produces an array of values.\n\nCommon functions:\n- range(end=5) -> [0, 1, 2, 3, 4]\n- range(start=1, end=6) -> [1, 2, 3, 4, 5]\n- pages(within='/blog') | map(attribute='slug') | list",
                expr,
                resolved_value.as_ref().map(|v| format!("\n\nThe expression resolved to:\n{}", v)).unwrap_or_default()
            );
            (reason, help, resolved_value)
        };

        HugsError::DynamicExprEval {
            file: source_path.display().to_string().into(),
            param_name: param_name.into(),
            expression: expr.to_string(),
            reason: display_reason,
            src: Some(NamedSource::new(
                source_path.display().to_string(),
                file_content.to_string(),
            )),
            span,
            resolved_value: resolved,
            help_text,
        }
    };

    let mapping = frontmatter.as_mapping().ok_or_else(|| HugsError::DynamicMissingParam {
        file: source_path.display().to_string().into(),
        param_name: param_name.into(),
    })?;

    let param_value = mapping
        .get(&YamlValue::String(param_name.to_string()))
        .ok_or_else(|| HugsError::DynamicMissingParam {
            file: source_path.display().to_string().into(),
            param_name: param_name.into(),
        })?;

    match param_value {
        // Direct array: page_no: [1, 2, 3]
        YamlValue::Sequence(seq) => Ok(seq.clone()),

        // Jinja expression: page_no: "{{ range(end=5) }}" or page_no: "range(end=5)"
        YamlValue::String(expr) => {
            // Create MiniJinja environment with helper functions
            let mut env = Environment::new();

            // Add the pages() function
            env.add_function("pages", create_pages_function(Arc::clone(pages)));

            // Collect function names for help() function (before adding help)
            let function_names: Vec<String> = env.globals().map(|(name, _)| name.to_string()).collect();

            // Add the help filter for debugging dynamic page expressions
            env.add_filter("help", create_help_filter());

            // Add the flatten filter for flattening nested sequences
            env.add_filter("flatten", create_flatten_filter());

            // Add the help test for debugging
            env.add_test("help", create_help_test());

            // Add the help function for debugging
            env.add_function("help", create_help_function(function_names));

            // Strip {{ }} wrapper if present (user can write either form)
            let clean_expr = expr
                .trim()
                .strip_prefix("{{")
                .and_then(|s| s.strip_suffix("}}"))
                .map(|s| s.trim())
                .unwrap_or(expr.trim());

            // Wrap expression to output JSON array
            // Use debug format for strings to get quoted output
            let template = format!(
                r#"{{% set result = {} %}}{{% for item in result %}}{{{{ item }}}}{{% if not loop.last %}}
{{% endif %}}{{% endfor %}}"#,
                clean_expr
            );

            // Collect available function names for error messages
            let available_functions: Vec<String> = env.globals().map(|(name, _)| name.to_string()).collect();

            env.add_template("expr", &template).map_err(|e| {
                make_error(expr, format_dynamic_expr_error(&e, &available_functions), None)
            })?;

            let tmpl = env.get_template("expr").map_err(|e| {
                make_error(expr, format_dynamic_expr_error(&e, &available_functions), None)
            })?;

            let output = tmpl.render(()).map_err(|e| {
                make_error(expr, format_dynamic_expr_error(&e, &available_functions), None)
            })?;

            // Parse the newline-separated output
            let values: Vec<YamlValue> = output
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    let trimmed = line.trim();
                    // Try to parse as number
                    if let Ok(i) = trimmed.parse::<i64>() {
                        YamlValue::Number(i.into())
                    } else if let Ok(f) = trimmed.parse::<f64>() {
                        YamlValue::Number(serde_yaml::Number::from(f))
                    } else if trimmed == "true" {
                        YamlValue::Bool(true)
                    } else if trimmed == "false" {
                        YamlValue::Bool(false)
                    } else {
                        YamlValue::String(trimmed.to_string())
                    }
                })
                .collect();

            Ok(values)
        }

        _ => Err(HugsError::DynamicParamParse {
            file: source_path.display().to_string().into(),
            param_name: param_name.into(),
            reason: "Parameter value must be an array or a Jinja expression string".into(),
        }),
    }
}

/// Format error message for dynamic expression evaluation, including available functions
fn format_dynamic_expr_error(error: &minijinja::Error, available_functions: &[String]) -> String {
    let base_msg = error
        .detail()
        .map(|s| s.to_string())
        .unwrap_or_else(|| error.to_string());

    // Check if this is an unknown function error
    if matches!(error.kind(), minijinja::ErrorKind::UnknownFunction) {
        format!(
            "{}. Available functions: {}",
            base_msg,
            available_functions.join(", ")
        )
    } else {
        base_msg
    }
}

/// Render template expressions in frontmatter values for dynamic pages.
///
/// This allows frontmatter like `title: "{{ tag | title }}"` to be evaluated
/// with the dynamic parameter context (e.g., tag = "basics" -> title = "Basics").
fn render_frontmatter_values(
    frontmatter: &YamlValue,
    dynamic_ctx: &DynamicContext,
    pages: &Arc<Vec<PageInfo>>,
    language: &str,
    source_file: &str,
    source_content: &str,
) -> Result<YamlValue> {
    let mapping = match frontmatter.as_mapping() {
        Some(m) => m,
        None => return Ok(frontmatter.clone()),
    };

    let mut env = Environment::new();

    // Add the pages() function
    env.add_function("pages", create_pages_function(Arc::clone(pages)));

    // Add the datefmt filter
    env.add_filter("datefmt", create_datefmt_filter(language.to_string()));

    // Add the help filter (same as in page templates)
    env.add_filter("help", create_help_filter());

    // Add the flatten filter for flattening nested sequences
    env.add_filter("flatten", create_flatten_filter());

    let mut rendered_mapping = serde_yaml::Mapping::new();

    for (key, value) in mapping {
        let rendered_value = render_yaml_value(value, &env, dynamic_ctx, source_file, source_content)?;
        rendered_mapping.insert(key.clone(), rendered_value);
    }

    Ok(YamlValue::Mapping(rendered_mapping))
}

/// Recursively render template expressions in a YAML value.
fn render_yaml_value(
    value: &YamlValue,
    env: &Environment,
    dynamic_ctx: &DynamicContext,
    source_file: &str,
    source_content: &str,
) -> Result<YamlValue> {
    match value {
        YamlValue::String(s) => {
            // Only render if it looks like it might contain template syntax
            if s.contains("{{") || s.contains("{%") {
                let rendered = render_single_template_string(s, env, dynamic_ctx, source_file, source_content)?;
                Ok(YamlValue::String(rendered))
            } else {
                Ok(value.clone())
            }
        }
        YamlValue::Sequence(seq) => {
            let rendered: Result<Vec<YamlValue>> = seq
                .iter()
                .map(|v| render_yaml_value(v, env, dynamic_ctx, source_file, source_content))
                .collect();
            Ok(YamlValue::Sequence(rendered?))
        }
        YamlValue::Mapping(map) => {
            let mut rendered_map = serde_yaml::Mapping::new();
            for (k, v) in map {
                rendered_map.insert(k.clone(), render_yaml_value(v, env, dynamic_ctx, source_file, source_content)?);
            }
            Ok(YamlValue::Mapping(rendered_map))
        }
        // Numbers, booleans, null - preserve as-is
        _ => Ok(value.clone()),
    }
}

/// Render a single template string with the dynamic context.
fn render_single_template_string(
    template_str: &str,
    env: &Environment,
    dynamic_ctx: &DynamicContext,
    source_file: &str,
    source_content: &str,
) -> Result<String> {
    let mut local_env = env.clone();

    // Generate hints from the environment for helpful error messages
    let hints = TemplateHints::from_environment(&local_env);

    // Find where the template string appears in the source content for error reporting
    let template_offset = source_content.find(template_str).unwrap_or(0);

    // Helper to create error with proper file location
    let make_error = |e: &minijinja::Error| {
        HugsError::frontmatter_template_error(
            source_file,
            source_content,
            template_str,
            template_offset,
            e,
            &hints,
        )
    };

    // Create a unique template name
    local_env
        .add_template("__frontmatter_value__", template_str)
        .map_err(|e| make_error(&e))?;

    let tmpl = local_env.get_template("__frontmatter_value__").map_err(|e| make_error(&e))?;

    // Create context with the dynamic parameter
    let (param_name, param_value) = dynamic_ctx.to_json_pair();
    let ctx = minijinja::context! {
        ..minijinja::Value::from_serialize(&serde_json::json!({
            param_name: param_value
        }))
    };

    tmpl.render(ctx).map_err(|e| make_error(&e))
}

/// Convert a YAML value to a string for URL generation
fn yaml_value_to_string(value: &YamlValue) -> String {
    match value {
        YamlValue::String(s) => s.clone(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::Bool(b) => b.to_string(),
        _ => format!("{:?}", value),
    }
}

/// Generate URL for a dynamic page instance
fn generate_dynamic_url(source_path: &Path, param_name: &str, value: &YamlValue) -> String {
    let path_str = source_path.with_extension("").to_string_lossy().to_string();
    let placeholder = format!("[{}]", param_name);
    let value_str = yaml_value_to_string(value);

    let replaced = path_str.replace(&placeholder, &value_str);

    if replaced == "index" {
        String::from("/")
    } else if replaced.ends_with("/index") {
        format!("/{}/", replaced.strip_suffix("/index").unwrap())
    } else {
        format!("/{}", replaced)
    }
}

/// Expand dynamic page definitions into concrete PageInfo entries
fn expand_dynamic_pages(dynamic_defs: &[DynamicPageDef]) -> Vec<PageInfo> {
    let mut expanded = Vec::new();

    for def in dynamic_defs {
        for value in &def.param_values {
            let url = generate_dynamic_url(&def.source_path, &def.param_name, value);

            // Create a copy of frontmatter with the parameter value set
            let mut frontmatter = def.frontmatter.clone();
            if let YamlValue::Mapping(ref mut map) = frontmatter {
                map.insert(
                    YamlValue::String(def.param_name.clone()),
                    value.clone(),
                );
            }

            expanded.push(PageInfo {
                url,
                file_path: def.source_path.to_string_lossy().to_string(),
                frontmatter,
            });
        }
    }

    expanded
}

/// Check if a string is a valid identifier for macro names
fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Convert a YAML value to a minijinja-compatible literal string
fn yaml_to_jinja_literal(value: &YamlValue) -> String {
    match value {
        YamlValue::Null => "none".to_string(),
        YamlValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        YamlValue::Sequence(seq) => {
            let items: Vec<String> = seq.iter().map(yaml_to_jinja_literal).collect();
            format!("[{}]", items.join(", "))
        }
        YamlValue::Mapping(map) => {
            let pairs: Vec<String> = map
                .iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        YamlValue::String(s) => s.clone(),
                        _ => return None,
                    };
                    Some(format!("\"{}\": {}", key, yaml_to_jinja_literal(v)))
                })
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        YamlValue::Tagged(t) => yaml_to_jinja_literal(&t.value),
    }
}

/// Parse a macro file into a MacroDefinition
fn parse_macro_file(path: &Path, content: &str) -> Result<MacroDefinition> {
    // Extract macro name from filename (card.md -> "card")
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| HugsError::MacroParse {
            file: path.into(),
            reason: "Could not extract filename".into(),
        })?
        .to_string();

    // Validate macro name is a valid identifier
    if !is_valid_identifier(&name) {
        return Err(HugsError::MacroInvalidName {
            path: path.into(),
            name: name.into(),
        });
    }

    // Parse frontmatter as YAML mapping
    let (frontmatter, body) = markdown_frontmatter::parse::<YamlValue>(content)
        .map_err(|e| HugsError::MacroParse {
            file: path.into(),
            reason: e.to_string(),
        })?;

    // Convert frontmatter to parameters
    let params = match &frontmatter {
        YamlValue::Mapping(m) => {
            m.iter()
                .filter_map(|(k, v)| {
                    let name = match k {
                        YamlValue::String(s) => s.clone(),
                        _ => return None,
                    };
                    let default_value = yaml_to_jinja_literal(v);
                    Some(MacroParam { name, default_value })
                })
                .collect()
        }
        _ => Vec::new(),
    };

    Ok(MacroDefinition {
        name,
        params,
        body: body.to_string(),
        source_path: path.to_path_buf(),
    })
}

/// Load all macro definitions from _/macros/*.md
async fn load_macros(site_path: &PathBuf) -> Result<Vec<MacroDefinition>> {
    let macros_dir = site_path.join("_/macros");
    if !macros_dir.exists() {
        return Ok(Vec::new());
    }

    let mut macros = Vec::new();

    for entry in WalkDir::new(&macros_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
    {
        let path = entry.path();
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            HugsError::MacroParse {
                file: path.into(),
                reason: format!("Could not read file: {}", e),
            }
        })?;
        let macro_def = parse_macro_file(path, &content)?;
        macros.push(macro_def);
    }

    Ok(macros)
}

/// Build a combined template string containing all macro definitions
fn build_macros_template(macros: &[MacroDefinition]) -> String {
    let mut template = String::new();

    for macro_def in macros {
        // Build parameter list with defaults
        let params_str: String = macro_def
            .params
            .iter()
            .map(|p| format!("{}={}", p.name, p.default_value))
            .collect::<Vec<_>>()
            .join(", ");

        template.push_str(&format!(
            "{{% macro {}({}) %}}\n{}\n{{% endmacro %}}\n\n",
            macro_def.name,
            params_str,
            macro_def.body.trim()
        ));
    }

    template
}

pub fn convert_file_path_to_url(path: &Path) -> String {
    let path_str = path.with_extension("").to_string_lossy().to_string();

    if path_str == "index" {
        // Root index.md -> /
        String::from("/")
    } else if path_str.ends_with("/index") {
        // Directory index.md -> /path/to/dir/ (with trailing slash for correct relative URL resolution)
        let dir_path = path_str.strip_suffix("/index").unwrap_or(&path_str);
        format!("/{}/", dir_path)
    } else {
        // Regular file -> /path/to/file
        format!("/{}", path_str)
    }
}

/// Intermediate result for parsing a single page file
enum ParsedPage {
    Static(PageInfo),
    RawDynamic(RawDynamicPageDef),
}

/// Phase 1: Scan all pages, collecting static pages and raw dynamic definitions
/// Dynamic parameter expressions are NOT evaluated here (they need pages to be available)
async fn scan_pages_raw(site_path: &PathBuf) -> Result<RawScanResult> {
    // 1. Collect paths synchronously (fast - just directory walking)
    let paths: Vec<(PathBuf, PathBuf)> = WalkDir::new(site_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .filter_map(|entry| {
            let path = entry.path();
            let relative_path = path.strip_prefix(site_path).ok()?;

            // Skip _ directory and [404].md
            if relative_path.starts_with("_") {
                return None;
            }
            if relative_path.to_string_lossy() == "[404].md" {
                return None;
            }

            Some((path.to_owned(), relative_path.to_owned()))
        })
        .collect();

    // 2. Read and parse files in parallel
    let mut join_set: JoinSet<Option<Result<ParsedPage>>> = JoinSet::new();

    for (path, relative_path) in paths {
        join_set.spawn(async move {
            let content = match tokio::fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(e) => {
                    console::warn(format!(
                        "couldn't read {}: {}, skipping",
                        relative_path.display(),
                        e
                    ));
                    return None;
                }
            };

            let frontmatter = match markdown_frontmatter::parse::<YamlValue>(&content) {
                Ok((fm, _body)) => fm,
                Err(e) => {
                    console::warn(format!(
                        "couldn't parse frontmatter in {}: {}, using empty metadata",
                        relative_path.display(),
                        e
                    ));
                    YamlValue::Mapping(serde_yaml::Mapping::new())
                }
            };

            // Check if this is a dynamic page
            if is_dynamic_page(&relative_path) {
                let filename = relative_path.file_name()?.to_str()?;
                let param_name = extract_param_name(filename)?;

                // Don't evaluate parameter values yet - we need pages to be available first
                Some(Ok(ParsedPage::RawDynamic(RawDynamicPageDef {
                    param_name,
                    source_path: relative_path,
                    frontmatter,
                    file_content: content,
                })))
            } else {
                let url = convert_file_path_to_url(&relative_path);
                let file_path = relative_path.to_string_lossy().to_string();

                Some(Ok(ParsedPage::Static(PageInfo {
                    url,
                    file_path,
                    frontmatter,
                })))
            }
        });
    }

    // 3. Collect results
    let mut static_pages = Vec::new();
    let mut raw_dynamic_defs = Vec::new();

    while let Some(result) = join_set.join_next().await {
        if let Ok(Some(parsed_result)) = result {
            match parsed_result? {
                ParsedPage::Static(page_info) => static_pages.push(page_info),
                ParsedPage::RawDynamic(def) => raw_dynamic_defs.push(def),
            }
        }
    }

    Ok(RawScanResult {
        static_pages,
        raw_dynamic_defs,
    })
}

/// Phase 2: Evaluate dynamic page parameters now that we have access to pages
fn evaluate_dynamic_defs(
    raw_defs: Vec<RawDynamicPageDef>,
    pages: &Arc<Vec<PageInfo>>,
) -> Result<Vec<DynamicPageDef>> {
    let mut evaluated_defs = Vec::new();

    for raw_def in raw_defs {
        let param_values = evaluate_param_values_with_pages(
            &raw_def.param_name,
            &raw_def.frontmatter,
            &raw_def.source_path,
            pages,
            &raw_def.file_content,
        )?;

        evaluated_defs.push(DynamicPageDef {
            param_name: raw_def.param_name,
            source_path: raw_def.source_path,
            param_values,
            frontmatter: raw_def.frontmatter,
        });
    }

    Ok(evaluated_defs)
}

#[derive(Serialize)]
pub struct PageContent<'a> {
    pub title: &'a str,
    pub header: &'a str,
    pub footer: &'a str,
    pub nav: &'a str,
    pub content: &'a str,
    pub main_content: &'a str,
    pub path_class: &'a str,
    pub base: &'a str,
    pub dev_script: &'a str,
    pub seo: SeoContext,
    pub syntax_highlighting_enabled: bool,
}



/// Resolve a URL path to a document, returning the frontmatter, HTML content, file path, and raw frontmatter JSON.
///
/// Returns:
/// - `Ok(Some(...))` if the page was found and rendered successfully
/// - `Ok(None)` if no page exists at this path (404)
/// - `Err(...)` if an error occurred while processing the page
pub async fn resolve_path_to_doc(
    path: &str,
    app_data: &AppData,
) -> Result<Option<(ContentFrontmatter, String, PathBuf, serde_json::Value)>> {
    let resolvable_path = {
        let check_path = if path.is_empty() { "index" } else { path };

        let mut possible_path = app_data.site_path.join(format!("{}.md", check_path));

        if possible_path.exists() {
            Some(possible_path)
        } else if check_path != "index" {
            possible_path = app_data.site_path.join(format!("{}/index.md", check_path));

            if possible_path.exists() {
                Some(possible_path)
            } else {
                None
            }
        } else {
            None
        }
    };

    let resolvable_path = match resolvable_path {
        Some(p) => p,
        None => return Ok(None),
    };

    let relative_path = resolvable_path
        .strip_prefix(&app_data.site_path)
        .unwrap_or(&resolvable_path);
    let relative_path_str = relative_path.display().to_string();

    let doc_content_jinja = tokio::fs::read_to_string(&resolvable_path)
        .await
        .with_file_read(&resolvable_path)?;

    let path_class = convert_path_to_class(&resolvable_path, app_data)?;

    // Parse frontmatter FIRST from raw content so it's available to the page body
    let (frontmatter, raw_body) =
        markdown_frontmatter::parse::<ContentFrontmatter>(&doc_content_jinja).map_err(|e| {
            HugsError::FrontmatterParse {
                file: relative_path_str.clone().into(),
                src: miette::NamedSource::new(relative_path_str.clone(), doc_content_jinja.clone()),
                span: miette::SourceSpan::from((0_usize, 1_usize)),
                reason: format!(
                    "I couldn't parse the frontmatter. Make sure you have a valid `title` field. Error: {}",
                    e
                ),
            }
        })?;

    let (raw_frontmatter, _) =
        markdown_frontmatter::parse::<YamlValue>(&doc_content_jinja).map_err(|e| {
            HugsError::FrontmatterParse {
                file: relative_path_str.clone().into(),
                src: miette::NamedSource::new(relative_path_str.clone(), doc_content_jinja.clone()),
                span: miette::SourceSpan::from((0_usize, 1_usize)),
                reason: format!("Failed to parse frontmatter as YAML: {}", e),
            }
        })?;
    let frontmatter_json = yaml_to_json_value(&raw_frontmatter);

    // Create merged context: PageContent fields + frontmatter fields
    let initial_page_content = PageContent {
        title: "",
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: "",
        main_content: "",
        path_class: &path_class,
        base: "/",
        dev_script: "",
        seo: SeoContext::default(),
        syntax_highlighting_enabled: false,
    };

    let mut context = serde_json::to_value(&initial_page_content).map_err(|e| HugsError::TemplateContext {
        reason: e.to_string(),
    })?;

    // Merge frontmatter into context so page body can access its own frontmatter
    if let (serde_json::Value::Object(ctx_map), serde_json::Value::Object(fm_map)) = (&mut context, &frontmatter_json) {
        for (key, value) in fm_map {
            ctx_map.insert(key.clone(), value.clone());
        }
    }

    // Render only the body (not frontmatter) with the merged context
    let body = render_template(raw_body, &context, &app_data.pages, None, &app_data.macros_template, app_data.config.build.reading_speed, &app_data.config.site.language)
        .map_err(|e| HugsError::template_render(
            &resolvable_path,
            raw_body,
            e.error,
            &e.hints,
            e.macro_prefix_bytes,
            e.macro_prefix_lines,
        ))?;

    let doc_html = markdown_to_html(&body, &app_data.config.build.syntax_highlighting)
        .map_err(|reason| HugsError::MarkdownParse {
            file: relative_path_str.into(),
            reason,
        })?;

    Ok(Some((frontmatter, doc_html, resolvable_path, frontmatter_json)))
}

/// Resolve a dynamic page from its source file path with dynamic context.
///
/// This is used for dynamic pages like `[slug].md` where we need to inject
/// the parameter value into the template context.
pub async fn resolve_dynamic_doc(
    source_file_path: &str,
    dynamic_ctx: &DynamicContext,
    app_data: &AppData,
) -> Result<(ContentFrontmatter, String, PathBuf, serde_json::Value)> {
    let resolvable_path = app_data.site_path.join(source_file_path);

    let relative_path_str = source_file_path.to_string();

    let doc_content_jinja = tokio::fs::read_to_string(&resolvable_path)
        .await
        .with_file_read(&resolvable_path)?;

    // For dynamic pages, use the param value in the path class (not the [param] placeholder)
    let value_str = yaml_value_to_string(&dynamic_ctx.param_value);
    let path_class = source_file_path
        .strip_suffix(".md")
        .unwrap_or(source_file_path)
        .replace(&format!("[{}]", dynamic_ctx.param_name), &value_str)
        .replace('/', " ");

    // Parse frontmatter as raw YAML first
    let (raw_frontmatter, raw_body) =
        markdown_frontmatter::parse::<YamlValue>(&doc_content_jinja).map_err(|e| {
            HugsError::FrontmatterParse {
                file: relative_path_str.clone().into(),
                src: miette::NamedSource::new(relative_path_str.clone(), doc_content_jinja.clone()),
                span: miette::SourceSpan::from((0_usize, 1_usize)),
                reason: format!("Failed to parse frontmatter as YAML: {}", e),
            }
        })?;

    // Render template expressions in frontmatter values (e.g., `title: "{{ tag | title }}"`)
    let rendered_frontmatter = render_frontmatter_values(
        &raw_frontmatter,
        dynamic_ctx,
        &app_data.pages,
        &app_data.config.site.language,
        &relative_path_str,
        &doc_content_jinja,
    )?;

    // Convert rendered frontmatter to JSON for template context
    let frontmatter_json = yaml_to_json_value(&rendered_frontmatter);

    // Deserialize rendered frontmatter into ContentFrontmatter
    let frontmatter: ContentFrontmatter = serde_yaml::from_value(rendered_frontmatter.clone())
        .map_err(|e| {
            HugsError::FrontmatterParse {
                file: relative_path_str.clone().into(),
                src: miette::NamedSource::new(relative_path_str.clone(), doc_content_jinja.clone()),
                span: miette::SourceSpan::from((0_usize, 1_usize)),
                reason: format!(
                    "I couldn't parse the frontmatter. Make sure you have a valid `title` field. Error: {}",
                    e
                ),
            }
        })?;

    // Create merged context: PageContent fields + frontmatter fields + dynamic parameter
    let initial_page_content = PageContent {
        title: "",
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: "",
        main_content: "",
        path_class: &path_class,
        base: "/",
        dev_script: "",
        seo: SeoContext::default(),
        syntax_highlighting_enabled: false,
    };

    let mut context = serde_json::to_value(&initial_page_content).map_err(|e| HugsError::TemplateContext {
        reason: e.to_string(),
    })?;

    // Merge frontmatter into context so page body can access its own frontmatter
    if let (serde_json::Value::Object(ctx_map), serde_json::Value::Object(fm_map)) = (&mut context, &frontmatter_json) {
        for (key, value) in fm_map {
            ctx_map.insert(key.clone(), value.clone());
        }
    }

    // Inject the dynamic parameter (e.g., `slug` = "hello")
    let (param_name, param_value) = dynamic_ctx.to_json_pair();
    if let serde_json::Value::Object(ref mut map) = context {
        map.insert(param_name, param_value);
    }

    // Render only the body (not frontmatter) with the merged context
    let body = render_template(raw_body, &context, &app_data.pages, None, &app_data.macros_template, app_data.config.build.reading_speed, &app_data.config.site.language)
        .map_err(|e| HugsError::template_render(
            &resolvable_path,
            raw_body,
            e.error,
            &e.hints,
            e.macro_prefix_bytes,
            e.macro_prefix_lines,
        ))?;

    let doc_html = markdown_to_html(&body, &app_data.config.build.syntax_highlighting)
        .map_err(|reason| HugsError::MarkdownParse {
            file: relative_path_str.into(),
            reason,
        })?;

    Ok((frontmatter, doc_html, resolvable_path, frontmatter_json))
}

pub async fn render_notfound_page(app_data: &AppData, dev_script: &str) -> Option<String> {
    let notfound_path = app_data.notfound_page.as_ref()?;

    let doc_content_jinja = tokio::fs::read_to_string(notfound_path).await.ok()?;

    // Parse frontmatter FIRST from raw content so it's available to the page body
    let (frontmatter, raw_body) = markdown_frontmatter::parse::<ContentFrontmatter>(&doc_content_jinja).ok()?;
    let (raw_frontmatter, _) = markdown_frontmatter::parse::<YamlValue>(&doc_content_jinja).ok()?;
    let frontmatter_json = yaml_to_json_value(&raw_frontmatter);

    // Create merged context: PageContent fields + frontmatter fields
    let initial_page_content = PageContent {
        title: "",
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: "",
        main_content: "",
        path_class: "notfound",
        base: "/",
        dev_script: "",
        seo: SeoContext::default(),
        syntax_highlighting_enabled: false,
    };

    let mut context = serde_json::to_value(&initial_page_content).ok()?;

    // Merge frontmatter into context so page body can access its own frontmatter
    if let (serde_json::Value::Object(ctx_map), serde_json::Value::Object(fm_map)) = (&mut context, &frontmatter_json) {
        for (key, value) in fm_map {
            ctx_map.insert(key.clone(), value.clone());
        }
    }

    // Render only the body (not frontmatter) with the merged context
    let body = render_template(raw_body, &context, &app_data.pages, None, &app_data.macros_template, app_data.config.build.reading_speed, &app_data.config.site.language).ok()?;

    let doc_html = markdown_to_html(&body, &app_data.config.build.syntax_highlighting).ok()?;

    let seo = build_seo_context(&frontmatter, "/404", &app_data.config.site);
    let rendered_title = render_title_template(&frontmatter.title, &app_data.config.site);

    let mut content_ctx = if let serde_json::Value::Object(map) = &frontmatter_json {
        serde_json::Value::Object(map.clone())
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };

    if let serde_json::Value::Object(ref mut map) = content_ctx {
        map.insert("content".to_string(), serde_json::Value::String(doc_html.clone()));
        map.insert("path_class".to_string(), serde_json::Value::String("notfound".to_string()));
        map.insert("base".to_string(), serde_json::Value::String("/".to_string()));
        map.insert("seo".to_string(), serde_json::to_value(&seo).unwrap_or(serde_json::Value::Null));
    }

    let content_template_rendered = render_template(
        &app_data.content_template,
        &content_ctx,
        &app_data.pages,
        None,
        &app_data.macros_template,
        app_data.config.build.reading_speed,
        &app_data.config.site.language,
    ).ok()?;

    let main_content_html = markdown::to_html_with_options(&content_template_rendered, &markdown_options()).ok()?;

    let content = PageContent {
        title: &rendered_title,
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: &doc_html,
        main_content: &main_content_html,
        path_class: "notfound",
        base: "/",
        dev_script,
        seo,
        syntax_highlighting_enabled: app_data.config.build.syntax_highlighting.enabled,
    };

    let cache_bust = app_data.cache_bust_function();
    let html_out = render_root_template(app_data, &content, &cache_bust).ok()?;

    Some(html_out)
}

pub async fn try_serve_static_file(path: &str, app_data: &AppData) -> Option<HttpResponse> {
    // Don't serve files from the _ directory as static assets
    if path.starts_with("_/") || path.starts_with("_") {
        return None;
    }

    let file_path = app_data.site_path.join(path);

    // Check if it's an actual file (not directory) and not a markdown file
    if file_path.is_file() {
        if let Some(ext) = file_path.extension() {
            if ext == "md" {
                return None; // Let markdown files be handled by the page renderer
            }
        }

        // Read and serve the file
        match tokio::fs::read(&file_path).await {
            Ok(contents) => {
                let mime_type = mime_guess::from_path(&file_path)
                    .first_or_octet_stream();

                Some(HttpResponse::Ok()
                    .content_type(ContentType(mime_type))
                    .body(contents))
            }
            Err(_) => None,
        }
    } else {
        None
    }
}

pub fn convert_path_to_base(path: &PathBuf, app_data: &AppData) -> Result<String> {
    let relative = path.strip_prefix(&app_data.site_path).map_err(|_| {
        HugsError::PathStripPrefix {
            path: path.into(),
            base: (&app_data.site_path).into(),
        }
    })?;

    // Get the parent directory of the file
    if let Some(parent) = relative.parent() {
        let parent_str = parent.to_string_lossy();
        if parent_str.is_empty() {
            Ok(String::from("/"))
        } else {
            Ok(format!("/{}/", parent_str))
        }
    } else {
        Ok(String::from("/"))
    }
}

pub fn convert_path_to_class(path: &PathBuf, app_data: &AppData) -> Result<String> {
    let relative = path.strip_prefix(&app_data.site_path).map_err(|_| {
        HugsError::PathStripPrefix {
            path: path.into(),
            base: (&app_data.site_path).into(),
        }
    })?;

    let without_ext = relative.with_extension("");

    // Strip "index" suffix - e.g., blog/index.md should have path_class "blog", not "blog index"
    let path_for_class = if without_ext.file_name().map(|f| f == "index").unwrap_or(false) {
        without_ext.parent().unwrap_or(&without_ext)
    } else {
        &without_ext
    };

    let path_str = path_for_class
        .to_str()
        .ok_or_else(|| HugsError::PathInvalidUtf8 {
            path: path.into(),
        })?
        .to_string()
        .replace("/", " ");

    // For root index.md, path_str will be empty - use "index" instead
    if path_str.is_empty() {
        Ok(String::from("index"))
    } else {
        Ok(path_str)
    }
}

/// Helper function to render a page to HTML
pub fn render_page_html(
    frontmatter: &ContentFrontmatter,
    frontmatter_json: &serde_json::Value,
    doc_html: &str,
    resolvable_path: &PathBuf,
    app_data: &AppData,
    dev_script: &str,
) -> Result<String> {
    let base = convert_path_to_base(resolvable_path, app_data)?;
    let path_class = convert_path_to_class(resolvable_path, app_data)?;
    let page_url = convert_file_path_to_url(
        resolvable_path
            .strip_prefix(&app_data.site_path)
            .unwrap_or(resolvable_path),
    );

    render_page_html_internal(frontmatter, frontmatter_json, doc_html, &page_url, &path_class, &base, app_data, dev_script)
}

/// Render a dynamic page to HTML with explicit URL (for proper SEO and path_class)
pub fn render_dynamic_page_html(
    frontmatter: &ContentFrontmatter,
    frontmatter_json: &serde_json::Value,
    doc_html: &str,
    page_url: &str,
    app_data: &AppData,
    dev_script: &str,
) -> Result<String> {
    // Derive base and path_class from the resolved URL instead of file path
    let url_path = page_url.trim_start_matches('/');
    let base = if url_path.is_empty() || url_path == "/" {
        String::from("/")
    } else if let Some(parent) = std::path::Path::new(url_path).parent() {
        let parent_str = parent.to_string_lossy();
        if parent_str.is_empty() {
            String::from("/")
        } else {
            format!("/{}/", parent_str)
        }
    } else {
        String::from("/")
    };

    let path_class = if url_path.is_empty() {
        String::from("index")
    } else {
        url_path.replace('/', " ")
    };

    render_page_html_internal(frontmatter, frontmatter_json, doc_html, page_url, &path_class, &base, app_data, dev_script)
}

/// Internal helper for rendering page HTML
fn render_page_html_internal(
    frontmatter: &ContentFrontmatter,
    frontmatter_json: &serde_json::Value,
    doc_html: &str,
    page_url: &str,
    path_class: &str,
    base: &str,
    app_data: &AppData,
    dev_script: &str,
) -> Result<String> {
    let seo = build_seo_context(frontmatter, page_url, &app_data.config.site);
    let rendered_title = render_title_template(&frontmatter.title, &app_data.config.site);

    let mut content_ctx = if let serde_json::Value::Object(map) = frontmatter_json {
        serde_json::Value::Object(map.clone())
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };

    if let serde_json::Value::Object(ref mut map) = content_ctx {
        map.insert("content".to_string(), serde_json::Value::String(doc_html.to_string()));
        map.insert("path_class".to_string(), serde_json::Value::String(path_class.to_string()));
        map.insert("base".to_string(), serde_json::Value::String(base.to_string()));
        map.insert("seo".to_string(), serde_json::to_value(&seo).unwrap_or(serde_json::Value::Null));
    }

    let content_template_rendered = render_template(
        &app_data.content_template,
        &content_ctx,
        &app_data.pages,
        None,
        &app_data.macros_template,
        app_data.config.build.reading_speed,
        &app_data.config.site.language,
    )
    .map_err(|e| HugsError::template_render_named(
        "_/content.md",
        &app_data.content_template,
        &e.error,
        &e.hints,
        e.macro_prefix_bytes,
        e.macro_prefix_lines,
    ))?;

    let main_content_html = markdown::to_html_with_options(&content_template_rendered, &markdown_options())
        .map_err(|e| HugsError::MarkdownParse {
            file: "_/content.md".into(),
            reason: e.to_string(),
        })?;

    let content = PageContent {
        title: &rendered_title,
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: doc_html,
        main_content: &main_content_html,
        path_class,
        base,
        dev_script,
        seo,
        syntax_highlighting_enabled: app_data.config.build.syntax_highlighting.enabled,
    };

    let cache_bust = app_data.cache_bust_function();
    render_root_template(app_data, &content, &cache_bust)
        .map_err(|e| HugsError::template_render_named(
            "root.jinja",
            ROOT_TEMPL,
            &e.error,
            &e.hints,
            e.macro_prefix_bytes,
            e.macro_prefix_lines,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_locale() {
        // Test underscore format
        assert!(parse_locale("en_US").is_some());
        assert!(parse_locale("fr_FR").is_some());
        assert!(parse_locale("de_DE").is_some());

        // Test hyphen format (should be normalized)
        assert!(parse_locale("en-US").is_some());
        assert!(parse_locale("fr-FR").is_some());

        // Invalid locale
        assert!(parse_locale("invalid").is_none());
    }

    #[test]
    fn test_parse_date_string() {
        // YYYY-MM-DD format
        assert!(parse_date_string_for_filter("2024-01-15").is_ok());

        // ISO 8601 format
        assert!(parse_date_string_for_filter("2024-01-15T10:30:00Z").is_ok());

        // YYYY-MM-DD HH:MM:SS format
        assert!(parse_date_string_for_filter("2024-01-15 10:30:00").is_ok());

        // Invalid format
        assert!(parse_date_string_for_filter("invalid").is_err());
        assert!(parse_date_string_for_filter("15/01/2024").is_err());
    }

    #[test]
    fn test_datefmt_filter_basic() {
        let mut env = Environment::new();
        env.add_filter("datefmt", create_datefmt_filter("en_US".to_string()));
        env.add_template("test", "{{ date | datefmt(\"%Y-%m-%d\") }}").unwrap();

        let tmpl = env.get_template("test").unwrap();
        let result = tmpl.render(minijinja::context! { date => "2024-01-15" }).unwrap();
        assert_eq!(result, "2024-01-15");
    }

    #[test]
    fn test_datefmt_filter_localized() {
        let mut env = Environment::new();
        env.add_filter("datefmt", create_datefmt_filter("en_US".to_string()));
        env.add_template("test", "{{ date | datefmt(\"%B\") }}").unwrap();

        let tmpl = env.get_template("test").unwrap();
        let result = tmpl.render(minijinja::context! { date => "2024-01-15" }).unwrap();
        assert_eq!(result, "January");
    }

    #[test]
    fn test_datefmt_filter_locale_override() {
        let mut env = Environment::new();
        env.add_filter("datefmt", create_datefmt_filter("en_US".to_string()));
        env.add_template("test", "{{ date | datefmt(\"%B\", locale=\"fr_FR\") }}").unwrap();

        let tmpl = env.get_template("test").unwrap();
        let result = tmpl.render(minijinja::context! { date => "2024-01-15" }).unwrap();
        assert_eq!(result, "janvier");
    }

    #[test]
    fn test_flatten_filter_basic() {
        let mut env = Environment::new();
        env.add_filter("flatten", create_flatten_filter());
        env.add_template("test", "{{ items | flatten | join(',') }}").unwrap();

        let tmpl = env.get_template("test").unwrap();
        let result = tmpl.render(minijinja::context! {
            items => vec![vec![1, 2], vec![3, 4]]
        }).unwrap();
        assert_eq!(result, "1,2,3,4");
    }

    #[test]
    fn test_flatten_filter_mixed() {
        let mut env = Environment::new();
        env.add_filter("flatten", create_flatten_filter());
        env.add_template("test", "{{ items | flatten | join(',') }}").unwrap();

        let tmpl = env.get_template("test").unwrap();
        let result = tmpl.render(minijinja::context! {
            items => vec![
                Value::from(1),
                Value::from(vec![2, 3]),
                Value::from(4)
            ]
        }).unwrap();
        assert_eq!(result, "1,2,3,4");
    }

    #[test]
    fn test_flatten_filter_empty() {
        let mut env = Environment::new();
        env.add_filter("flatten", create_flatten_filter());
        env.add_template("test", "{{ items | flatten | join(',') }}").unwrap();

        let tmpl = env.get_template("test").unwrap();
        let result = tmpl.render(minijinja::context! {
            items => Vec::<Vec<i32>>::new()
        }).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_dynamic_param_pages_function_available() {
        // Test that pages() function is available in dynamic parameter expressions
        // This allows frontmatter like: slug: "{{ pages(within='/blog/') | map(attribute='url') }}"
        let pages = Arc::new(vec![
            PageInfo {
                url: "/blog/post1".to_string(),
                file_path: "blog/post1.md".to_string(),
                frontmatter: YamlValue::Mapping(serde_yaml::Mapping::new()),
            },
            PageInfo {
                url: "/blog/post2".to_string(),
                file_path: "blog/post2.md".to_string(),
                frontmatter: YamlValue::Mapping(serde_yaml::Mapping::new()),
            },
        ]);

        // Expression using pages() to get URLs from blog posts
        let expr = "{{ pages(within='/blog/') | map(attribute='url') }}";
        let file_content = format!(r#"---
slug: "{}"
---

Content"#, expr);
        let mut frontmatter = serde_yaml::Mapping::new();
        frontmatter.insert(
            YamlValue::String("slug".to_string()),
            YamlValue::String(expr.to_string()),
        );
        let yaml_fm = YamlValue::Mapping(frontmatter);

        let result = evaluate_param_values_with_pages(
            "slug",
            &yaml_fm,
            Path::new("test/[slug].md"),
            &pages,
            &file_content,
        );

        assert!(result.is_ok(), "pages() should be available in frontmatter expressions: {:?}", result.err());
        let values = result.unwrap();
        // Should produce two URLs from the blog posts
        assert_eq!(values.len(), 2);
        assert!(values.contains(&YamlValue::String("/blog/post1".to_string())));
        assert!(values.contains(&YamlValue::String("/blog/post2".to_string())));
    }

    #[test]
    fn test_dynamic_param_error_includes_environment_info() {
        // Test that errors from frontmatter evaluation include helpful environment info
        let pages = Arc::new(vec![
            PageInfo {
                url: "/blog/post1".to_string(),
                file_path: "blog/post1.md".to_string(),
                frontmatter: YamlValue::Mapping(serde_yaml::Mapping::new()),
            },
        ]);

        // Expression with a typo (pges instead of pages)
        let expr = "{{ pges(within='/blog/') }}";
        let file_content = format!(r#"---
slug: "{}"
---

Content"#, expr);
        let mut frontmatter = serde_yaml::Mapping::new();
        frontmatter.insert(
            YamlValue::String("slug".to_string()),
            YamlValue::String(expr.to_string()),
        );
        let yaml_fm = YamlValue::Mapping(frontmatter);

        let result = evaluate_param_values_with_pages(
            "slug",
            &yaml_fm,
            Path::new("test/[slug].md"),
            &pages,
            &file_content,
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{:?}", err);

        // Error should mention the available functions
        assert!(
            err_str.contains("pages") || err_str.contains("Available functions"),
            "Error should mention available functions like 'pages'. Got: {}",
            err_str
        );
    }

    #[test]
    fn test_render_frontmatter_values_with_dynamic_context() {
        // Test that dynamic frontmatter values like `title: "{{ tag | title }}"`
        // are rendered with the dynamic parameter context
        let pages = Arc::new(vec![]);

        // Create frontmatter with template expressions
        let mut frontmatter = serde_yaml::Mapping::new();
        frontmatter.insert(
            YamlValue::String("title".to_string()),
            YamlValue::String("{{ tag | title }}".to_string()),
        );
        frontmatter.insert(
            YamlValue::String("description".to_string()),
            YamlValue::String("Posts tagged with {{ tag }}".to_string()),
        );
        frontmatter.insert(
            YamlValue::String("static_field".to_string()),
            YamlValue::String("No template here".to_string()),
        );
        let yaml_fm = YamlValue::Mapping(frontmatter);

        // Create dynamic context with tag = "basics"
        let dynamic_ctx = DynamicContext {
            param_name: "tag".to_string(),
            param_value: YamlValue::String("basics".to_string()),
        };

        let result = render_frontmatter_values(
            &yaml_fm,
            &dynamic_ctx,
            &pages,
            "en_US",
            "test.md",
            "---\ntitle: \"{{ tag | title }}\"\n---\n",
        );

        assert!(result.is_ok(), "render_frontmatter_values should succeed: {:?}", result.err());
        let rendered = result.unwrap();

        // Check that the title was rendered with the `title` filter
        if let YamlValue::Mapping(map) = &rendered {
            let title = map.get(&YamlValue::String("title".to_string()));
            assert_eq!(
                title,
                Some(&YamlValue::String("Basics".to_string())),
                "title should be rendered as 'Basics'"
            );

            let description = map.get(&YamlValue::String("description".to_string()));
            assert_eq!(
                description,
                Some(&YamlValue::String("Posts tagged with basics".to_string())),
                "description should be rendered with tag value"
            );

            let static_field = map.get(&YamlValue::String("static_field".to_string()));
            assert_eq!(
                static_field,
                Some(&YamlValue::String("No template here".to_string())),
                "static fields should remain unchanged"
            );
        } else {
            panic!("Expected Mapping, got {:?}", rendered);
        }
    }

    #[test]
    fn test_render_frontmatter_values_preserves_non_string_values() {
        // Test that non-string values (numbers, arrays, etc.) are preserved
        let pages = Arc::new(vec![]);

        let mut frontmatter = serde_yaml::Mapping::new();
        frontmatter.insert(
            YamlValue::String("title".to_string()),
            YamlValue::String("{{ tag | title }}".to_string()),
        );
        frontmatter.insert(
            YamlValue::String("order".to_string()),
            YamlValue::Number(42.into()),
        );
        frontmatter.insert(
            YamlValue::String("tags".to_string()),
            YamlValue::Sequence(vec![
                YamlValue::String("rust".to_string()),
                YamlValue::String("web".to_string()),
            ]),
        );
        let yaml_fm = YamlValue::Mapping(frontmatter);

        let dynamic_ctx = DynamicContext {
            param_name: "tag".to_string(),
            param_value: YamlValue::String("basics".to_string()),
        };

        let result = render_frontmatter_values(
            &yaml_fm,
            &dynamic_ctx,
            &pages,
            "en_US",
            "test.md",
            "---\ntitle: \"{{ tag | title }}\"\norder: 42\n---\n",
        );

        assert!(result.is_ok());
        let rendered = result.unwrap();

        if let YamlValue::Mapping(map) = &rendered {
            // Number should be preserved
            let order = map.get(&YamlValue::String("order".to_string()));
            assert_eq!(order, Some(&YamlValue::Number(42.into())));

            // Array should be preserved
            let tags = map.get(&YamlValue::String("tags".to_string()));
            assert_eq!(
                tags,
                Some(&YamlValue::Sequence(vec![
                    YamlValue::String("rust".to_string()),
                    YamlValue::String("web".to_string()),
                ]))
            );
        } else {
            panic!("Expected Mapping, got {:?}", rendered);
        }
    }

    #[test]
    fn test_render_frontmatter_unknown_filter_returns_proper_error() {
        // Test that unknown filters in frontmatter return a TemplateRender error
        // with helpful suggestions, not a generic TemplateContext error
        let pages = Arc::new(vec![]);

        let mut frontmatter = serde_yaml::Mapping::new();
        frontmatter.insert(
            YamlValue::String("title".to_string()),
            YamlValue::String("{{ tag | unknownfilter }}".to_string()),
        );
        let yaml_fm = YamlValue::Mapping(frontmatter);

        let dynamic_ctx = DynamicContext {
            param_name: "tag".to_string(),
            param_value: YamlValue::String("basics".to_string()),
        };

        let result = render_frontmatter_values(
            &yaml_fm,
            &dynamic_ctx,
            &pages,
            "en_US",
            "test.md",
            "---\ntitle: \"{{ tag | unknownfilter }}\"\n---\n",
        );

        assert!(result.is_err(), "Should fail with unknown filter");
        let err = result.unwrap_err();

        // Check that the error is a TemplateRender error, not TemplateContext
        match &err {
            HugsError::TemplateRender { help_text, .. } => {
                // Should mention it's an unknown filter and suggest alternatives
                assert!(
                    help_text.contains("filter") || help_text.contains("Filter"),
                    "Error should mention filters. Got help_text: {}",
                    help_text
                );
            }
            HugsError::TemplateContext { reason } => {
                panic!(
                    "Got TemplateContext error instead of TemplateRender. \
                     TemplateContext errors are generic and unhelpful. \
                     Reason: {}",
                    reason
                );
            }
            other => {
                panic!("Expected TemplateRender error, got: {:?}", other);
            }
        }
    }

    #[test]
    fn test_render_frontmatter_help_filter_works() {
        // Test that the help filter is available in frontmatter and produces
        // the expected help output (not an "unknown filter" error)
        let pages = Arc::new(vec![]);

        let mut frontmatter = serde_yaml::Mapping::new();
        frontmatter.insert(
            YamlValue::String("title".to_string()),
            YamlValue::String("{{ tag | help }}".to_string()),
        );
        let yaml_fm = YamlValue::Mapping(frontmatter);

        let dynamic_ctx = DynamicContext {
            param_name: "tag".to_string(),
            param_value: YamlValue::String("basics".to_string()),
        };

        let result = render_frontmatter_values(
            &yaml_fm,
            &dynamic_ctx,
            &pages,
            "en_US",
            "test.md",
            "---\ntitle: \"{{ tag | help }}\"\n---\n",
        );

        // The help filter should error (as designed), but the error should
        // show helpful filter information, not an "unknown filter" error
        assert!(result.is_err(), "Help filter should produce an error");
        let err = result.unwrap_err();

        // Check that we got a TemplateRender error with the help output
        match &err {
            HugsError::TemplateRender { reason, help_text, .. } => {
                // The reason should indicate this is a help request
                assert!(
                    reason.contains("you asked for filter help"),
                    "Error reason should indicate help was requested. Got: {}",
                    reason
                );
                // The help text should show available filters
                assert!(
                    help_text.contains("Filters you can apply"),
                    "Help text should list available filters. Got: {}",
                    help_text
                );
                // The help text should show the value being filtered
                assert!(
                    help_text.contains("basics") || help_text.contains("String"),
                    "Help text should show the value type. Got: {}",
                    help_text
                );
            }
            other => {
                panic!("Expected TemplateRender error with help output, got: {:?}", other);
            }
        }
    }

    #[test]
    fn test_render_frontmatter_error_shows_file_location() {
        // Test that frontmatter template errors show the actual file path and line number
        let pages = Arc::new(vec![]);

        let mut frontmatter = serde_yaml::Mapping::new();
        frontmatter.insert(
            YamlValue::String("title".to_string()),
            YamlValue::String("{{ tag | help }}".to_string()),
        );
        let yaml_fm = YamlValue::Mapping(frontmatter);

        let dynamic_ctx = DynamicContext {
            param_name: "tag".to_string(),
            param_value: YamlValue::String("basics".to_string()),
        };

        let source_file = "blog/[tag].md";
        let source_content = "---\ntitle: \"{{ tag | help }}\"\ndescription: \"Test\"\n---\n\nContent here";

        let result = render_frontmatter_values(
            &yaml_fm,
            &dynamic_ctx,
            &pages,
            "en_US",
            source_file,
            source_content,
        );

        assert!(result.is_err(), "Help filter should produce an error");
        let err = result.unwrap_err();

        // Check that the error shows the actual file path, not "frontmatter"
        match &err {
            HugsError::TemplateRender { file, src, span, .. } => {
                // The file path should be the actual source file
                let file_str = format!("{:?}", file);
                assert!(
                    file_str.contains("blog/[tag].md"),
                    "Error should show actual file path 'blog/[tag].md', got: {}",
                    file_str
                );

                // The source name should be the file path (not "frontmatter")
                let src_str = format!("{:?}", src);
                assert!(
                    src_str.contains("blog/[tag].md"),
                    "Error source name should be the file path, got: {}",
                    src_str
                );

                // The span should point to somewhere in the file (not just offset 0)
                // The template "{{ tag | help }}" starts after "---\ntitle: \"" which is 14 bytes
                let offset: usize = (*span).offset().into();
                assert!(
                    offset >= 14,
                    "Error span should point into the file content (after frontmatter header), got offset: {}",
                    offset
                );
            }
            other => {
                panic!("Expected TemplateRender error, got: {:?}", other);
            }
        }
    }

    #[test]
    fn test_dynamic_expr_eval_error_shows_source_span() {
        // Test that DynamicExprEval errors include source span pointing to the expression
        let pages = Arc::new(vec![
            PageInfo {
                url: "/blog/post1".to_string(),
                file_path: "blog/post1.md".to_string(),
                frontmatter: YamlValue::Mapping(serde_yaml::Mapping::new()),
            },
        ]);

        // Expression that will fail - using help filter which intentionally throws an error
        // to display help information
        let file_content = r#"---
title: "{{ tag | title }}"
tag: "{{ pages(within='/blog') | map(attribute='tags') | list | help }}"
---

Content here"#;

        let frontmatter = markdown_frontmatter::parse::<YamlValue>(file_content)
            .map(|(fm, _)| fm)
            .unwrap();

        let source_path = Path::new("blog/[tag].md");

        let result = evaluate_param_values_with_pages(
            "tag",
            &frontmatter,
            source_path,
            &pages,
            file_content,
        );

        assert!(result.is_err(), "Expression with |help should fail as it throws an error");
        let err = result.unwrap_err();

        // Check that the error includes source span information
        match &err {
            HugsError::DynamicExprEval { file, src, span, expression, help_text, .. } => {
                // Should show the file path
                let file_str = format!("{:?}", file);
                assert!(
                    file_str.contains("blog/[tag].md"),
                    "Error should show file path 'blog/[tag].md', got: {}",
                    file_str
                );

                // Should have source code attached
                assert!(
                    src.is_some(),
                    "Error should include source code for display"
                );

                // Span should point to the expression in the frontmatter
                let offset: usize = (*span).offset().into();
                // The tag: field starts around position 32 in the content
                assert!(
                    offset > 20 && offset < 100,
                    "Error span should point to the tag expression in frontmatter, got offset: {}",
                    offset
                );

                // Should show the expression
                assert!(
                    expression.contains("pages(within='/blog')"),
                    "Error should include the expression, got: {}",
                    expression
                );

                // Should have helpful context about what Hugs was trying to do
                assert!(
                    help_text.contains("trying to determine the routes"),
                    "Help text should explain Hugs was determining routes. Got: {}",
                    help_text
                );
            }
            other => {
                panic!("Expected DynamicExprEval error, got: {:?}", other);
            }
        }
    }

    #[test]
    fn test_dynamic_expr_eval_error_shows_source_span_for_unknown_function() {
        // Test that errors from unknown functions include source span
        let pages = Arc::new(vec![]);

        // Expression with typo (unknownfunc instead of a real function)
        let file_content = r#"---
title: "Test"
slug: "{{ unknownfunc() }}"
---

Content"#;

        let frontmatter = markdown_frontmatter::parse::<YamlValue>(file_content)
            .map(|(fm, _)| fm)
            .unwrap();

        let source_path = Path::new("test/[slug].md");

        let result = evaluate_param_values_with_pages(
            "slug",
            &frontmatter,
            source_path,
            &pages,
            file_content,
        );

        assert!(result.is_err(), "Expression with unknown function should fail");
        let err = result.unwrap_err();

        match &err {
            HugsError::DynamicExprEval { src, span, file, .. } => {
                // Should have source code
                assert!(src.is_some(), "Error should include source code");

                // Should show the file path
                let file_str = format!("{:?}", file);
                assert!(
                    file_str.contains("test/[slug].md"),
                    "Error should show file path, got: {}",
                    file_str
                );

                // Span should point into the file
                let offset: usize = (*span).offset().into();
                assert!(
                    offset > 0,
                    "Span should point to the expression, got offset: {}",
                    offset
                );
            }
            other => {
                panic!("Expected DynamicExprEval error, got: {:?}", other);
            }
        }
    }

    #[test]
    fn test_dynamic_expr_help_filter_is_recognized() {
        // Test that the |help filter is recognized in dynamic page expressions
        // and produces helpful debug output (not "filter help is unknown")
        let pages = Arc::new(vec![
            PageInfo {
                url: "/blog/post1".to_string(),
                file_path: "blog/post1.md".to_string(),
                frontmatter: YamlValue::Mapping(serde_yaml::Mapping::new()),
            },
        ]);

        // Expression using |help filter - should be recognized and produce help output
        let file_content = r#"---
title: "Test"
tag: "{{ pages(within='/blog') | help }}"
---

Content"#;

        let frontmatter = markdown_frontmatter::parse::<YamlValue>(file_content)
            .map(|(fm, _)| fm)
            .unwrap();

        let source_path = Path::new("blog/[tag].md");

        let result = evaluate_param_values_with_pages(
            "tag",
            &frontmatter,
            source_path,
            &pages,
            file_content,
        );

        // The help filter intentionally throws an error to display help info
        assert!(result.is_err(), "Expression with |help should fail (to show help)");
        let err = result.unwrap_err();

        match &err {
            HugsError::DynamicExprEval { reason, help_text, resolved_value, .. } => {
                // The reason (span label) should match template errors
                assert!(
                    reason.contains("you asked for filter help here"),
                    "Help filter should use same span label as template errors. Got: {}",
                    reason
                );

                // Help text should say "You're filtering a <type>"
                assert!(
                    help_text.contains("You're filtering a"),
                    "Help text should match template error format. Got: {}",
                    help_text
                );

                // Help text should show available filters (like page template help)
                assert!(
                    help_text.contains("Filters you can apply:"),
                    "Help text should list available filters. Got: {}",
                    help_text
                );

                // Verify some specific filters are listed
                assert!(
                    help_text.contains("map") && help_text.contains("select") && help_text.contains("join"),
                    "Help text should include common filters like map, select, join. Got: {}",
                    help_text
                );

                // resolved_value should contain the decoded array
                assert!(
                    resolved_value.is_some(),
                    "resolved_value should be set when |help is used"
                );
            }
            other => {
                panic!("Expected DynamicExprEval error, got: {:?}", other);
            }
        }
    }

    #[test]
    fn test_dynamic_expr_help_test_is_recognized() {
        // Test that the `is help` test is recognized in dynamic page expressions
        // and produces helpful debug output (not "test help is unknown")
        let pages = Arc::new(vec![
            PageInfo {
                url: "/blog/post1".to_string(),
                file_path: "blog/post1.md".to_string(),
                frontmatter: YamlValue::Mapping(serde_yaml::Mapping::new()),
            },
        ]);

        // Expression using `is help` test - should be recognized and produce help output
        // Note: We use selectattr which applies the test to each item
        let file_content = r#"---
title: "Test"
tag: "{{ pages(within='/blog') | selectattr('url', 'help') | list }}"
---

Content"#;

        let frontmatter = markdown_frontmatter::parse::<YamlValue>(file_content)
            .map(|(fm, _)| fm)
            .unwrap();

        let source_path = Path::new("blog/[tag].md");

        let result = evaluate_param_values_with_pages(
            "tag",
            &frontmatter,
            source_path,
            &pages,
            file_content,
        );

        // The help test intentionally throws an error to display help info
        assert!(result.is_err(), "Expression with `is help` should fail (to show help)");
        let err = result.unwrap_err();

        match &err {
            HugsError::DynamicExprEval { reason, help_text, resolved_value, .. } => {
                // The reason (span label) should match template errors
                assert!(
                    reason.contains("you asked for test help here"),
                    "Help test should use same span label as template errors. Got: {}",
                    reason
                );

                // Help text should say "You're testing a <type>"
                assert!(
                    help_text.contains("You're testing a"),
                    "Help text should match template error format. Got: {}",
                    help_text
                );

                // Help text should show available tests (like page template help)
                assert!(
                    help_text.contains("Tests you can use:"),
                    "Help text should list available tests. Got: {}",
                    help_text
                );

                // Verify some specific tests are listed
                assert!(
                    help_text.contains("defined") && help_text.contains("sequence") && help_text.contains("even"),
                    "Help text should include common tests like defined, sequence, even. Got: {}",
                    help_text
                );

                // resolved_value should contain the decoded value
                assert!(
                    resolved_value.is_some(),
                    "resolved_value should be set when `is help` is used"
                );
            }
            other => {
                panic!("Expected DynamicExprEval error, got: {:?}", other);
            }
        }
    }

    #[test]
    fn test_dynamic_expr_help_function_is_recognized() {
        // Test that the help() function is recognized in dynamic page expressions
        // and produces helpful debug output (not "function help is unknown")
        let pages = Arc::new(vec![
            PageInfo {
                url: "/blog/post1".to_string(),
                file_path: "blog/post1.md".to_string(),
                frontmatter: YamlValue::Mapping(serde_yaml::Mapping::new()),
            },
        ]);

        // Expression using help() function - should be recognized and produce help output
        let file_content = r#"---
title: "Test"
tag: "{{ help() }}"
---

Content"#;

        let frontmatter = markdown_frontmatter::parse::<YamlValue>(file_content)
            .map(|(fm, _)| fm)
            .unwrap();

        let source_path = Path::new("blog/[tag].md");

        let result = evaluate_param_values_with_pages(
            "tag",
            &frontmatter,
            source_path,
            &pages,
            file_content,
        );

        // The help function intentionally throws an error to display help info
        assert!(result.is_err(), "Expression with help() should fail (to show help)");
        let err = result.unwrap_err();

        match &err {
            HugsError::DynamicExprEval { reason, help_text, .. } => {
                // The reason (span label) should match template errors
                assert!(
                    reason.contains("you asked for help here"),
                    "Help function should use same span label as template errors. Got: {}",
                    reason
                );

                // Help text should show variables section with explanation
                assert!(
                    help_text.contains("Variables you can use:"),
                    "Help text should have variables section. Got: {}",
                    help_text
                );
                assert!(
                    help_text.contains("no variables are pre-defined"),
                    "Help text should explain no pre-defined variables. Got: {}",
                    help_text
                );

                // Help text should show functions section
                assert!(
                    help_text.contains("Functions you can call:"),
                    "Help text should have functions section. Got: {}",
                    help_text
                );
                assert!(
                    help_text.contains("pages()"),
                    "Help text should list pages() function. Got: {}",
                    help_text
                );

                // Help text should show filters section
                assert!(
                    help_text.contains("Filters you can apply:"),
                    "Help text should have filters section. Got: {}",
                    help_text
                );
                assert!(
                    help_text.contains("map") && help_text.contains("select"),
                    "Help text should include common filters. Got: {}",
                    help_text
                );

                // Help text should show tests section
                assert!(
                    help_text.contains("Tests you can use:"),
                    "Help text should have tests section. Got: {}",
                    help_text
                );
                assert!(
                    help_text.contains("defined") && help_text.contains("sequence"),
                    "Help text should include common tests. Got: {}",
                    help_text
                );
            }
            other => {
                panic!("Expected DynamicExprEval error, got: {:?}", other);
            }
        }
    }
}
