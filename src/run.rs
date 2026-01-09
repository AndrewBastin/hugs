use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use actix_web::{HttpResponse, http::header::ContentType};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml::Value as YamlValue;
use sha2::{Sha256, Digest};
use tera::{Context, Function, Tera, Value, to_value};
use tokio::task::JoinSet;
use tracing::warn;
use walkdir::WalkDir;

use crate::config::SiteConfig;
use crate::error::{HugsError, HugsResultExt, Result};

/// Create markdown options (can't be static due to non-Send callback fields)
fn markdown_options() -> markdown::Options {
    markdown::Options {
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

struct PagesFunction {
    pages: Arc<Vec<PageInfo>>,
}

impl Function for PagesFunction {
    fn call(&self, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let pages_value = to_value(&*self.pages).unwrap();

        // If `within` arg is provided, filter by URL prefix
        if let Some(within) = args.get("within") {
            let prefix = within.as_str().ok_or_else(|| {
                tera::Error::msg("Function `pages` argument `within` must be a string")
            })?;

            let arr = pages_value.as_array().unwrap();
            // The index URL for the directory is the prefix with a trailing slash
            let index_url = if prefix.ends_with('/') {
                prefix.to_string()
            } else {
                format!("{}/", prefix)
            };
            let filtered: Vec<Value> = arr
                .iter()
                .filter(|item| {
                    if let Some(url) = item.get("url").and_then(|u| u.as_str()) {
                        // Include pages within the prefix, but exclude the directory index
                        url.starts_with(prefix) && url != index_url
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            return Ok(to_value(filtered).unwrap());
        }

        Ok(pages_value)
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

/// Tera function for opt-in cache busting via content hashing.
/// Usage: {{ cache_bust(path="/theme.css") }} -> "/theme.a1b2c3f4.css"
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
}

impl Function for CacheBustFunction {
    fn call(&self, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let path = args
            .get("path")
            .ok_or_else(|| tera::Error::msg("Function `cache_bust` requires `path` argument"))?
            .as_str()
            .ok_or_else(|| tera::Error::msg("Function `cache_bust` argument `path` must be a string"))?;

        // Check if already computed
        {
            let entries = self.registry.entries.lock().unwrap();
            if let Some(hashed) = entries.get(path) {
                return Ok(Value::String(hashed.clone()));
            }
        }

        // Get content (special case for theme.css and highlight.css which are pre-loaded)
        let content = if path == "/theme.css" {
            self.theme_css.as_bytes().to_vec()
        } else if path == "/highlight.css" {
            self.highlight_css.as_bytes().to_vec()
        } else {
            let file_path = if path.starts_with('/') {
                self.site_path.join(&path[1..])
            } else {
                self.site_path.join(path)
            };
            std::fs::read(&file_path).map_err(|e| {
                tera::Error::msg(format!("cache_bust: cannot read file '{}': {}", path, e))
            })?
        };

        // Compute hash (first 8 hex chars of SHA-256)
        let hash = compute_content_hash(&content);
        let hashed_path = insert_hash_into_path(path, &hash);

        // Register for build phase
        self.registry.insert(path, &hashed_path);

        Ok(Value::String(hashed_path))
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

pub const ROOT_TEMPL: &'static str = include_str!("templates/root.tera");

pub fn render_template(
    template: &str,
    context: &Context,
    pages: &Arc<Vec<PageInfo>>,
    cache_bust: Option<&CacheBustFunction>,
) -> std::result::Result<String, tera::Error> {
    let mut tera = Tera::default();
    tera.register_function("pages", PagesFunction { pages: Arc::clone(pages) });
    if let Some(cb) = cache_bust {
        tera.register_function("cache_bust", cb.clone());
    }
    tera.add_raw_template("template", template)?;
    tera.render("template", context)
}

/// Render using the pre-compiled root template (avoids re-parsing ROOT_TEMPL)
pub fn render_root_template(
    app_data: &AppData,
    context: &Context,
    cache_bust: &CacheBustFunction,
) -> std::result::Result<String, tera::Error> {
    // Clone the pre-compiled Tera instance (cheap - internal data is Arc-wrapped)
    let mut tera = app_data.root_tera.clone();
    tera.register_function("pages", PagesFunction { pages: Arc::clone(&app_data.pages) });
    tera.register_function("cache_bust", cache_bust.clone());
    tera.render("root", context)
}

fn parse_md(
    content_tera_md: &str,
    page_content: &PageContent<'_>,
    pages: &Arc<Vec<PageInfo>>,
    source_name: &str,
) -> Result<String> {
    let context = Context::from_serialize(page_content).map_err(|e| HugsError::TemplateContext {
        reason: e.to_string(),
    })?;

    let content_md = render_template(content_tera_md, &context, pages, None)
        .map_err(|e| HugsError::template_render_named(source_name, content_tera_md, &e))?;

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

    /// Pre-compiled root template for efficient page rendering
    pub root_tera: Tera,

    /// Pre-generated CSS for syntax highlighting
    pub highlight_css: String,
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
    pub async fn load(site_path: PathBuf) -> Result<AppData> {
        // Check if the site directory exists first
        if !site_path.is_dir() {
            return Err(HugsError::SiteNotFound {
                path: (&site_path).into(),
            });
        }

        let header_path = site_path.join("_/header.md");
        let footer_path = site_path.join("_/footer.md");
        let nav_path = site_path.join("_/nav.md");
        let theme_path = site_path.join("_/theme.css");

        let header_md = read_required_file(&header_path, "header", "_/header.md").await?;
        let footer_md = read_required_file(&footer_path, "footer", "_/footer.md").await?;
        let nav_md = read_required_file(&nav_path, "navigation", "_/nav.md").await?;
        let theme_css = read_required_file(&theme_path, "theme stylesheet", "_/theme.css").await?;
        let config = SiteConfig::load(&site_path).await?;

        // Initialize syntax highlighting registry and generate CSS
        crate::highlight::init_registry();
        let highlight_css = if config.build.syntax_highlighting.enabled {
            crate::highlight::generate_theme_css(&config.build.syntax_highlighting.theme)
        } else {
            String::new()
        };

        // Scan pages and separate static from dynamic
        let scan_result = scan_pages(&site_path).await?;

        // Expand dynamic pages into concrete pages
        let expanded_pages = expand_dynamic_pages(&scan_result.dynamic_defs);

        // Combine static and expanded pages
        let mut all_pages = scan_result.static_pages;
        all_pages.extend(expanded_pages);

        let pages = Arc::new(all_pages);
        let dynamic_defs = Arc::new(scan_result.dynamic_defs);

        let initial_page_content = PageContent {
            title: "",
            header: "",
            footer: "",
            nav: "",
            content: "",
            path_class: "",
            base: "/",
            dev_script: "",
            seo: SeoContext::default(),
            syntax_highlighting_enabled: false,
        };

        let header_html = parse_md(&header_md, &initial_page_content, &pages, "_/header.md")?;
        let footer_html = parse_md(&footer_md, &initial_page_content, &pages, "_/footer.md")?;
        let nav_html = parse_md(&nav_md, &initial_page_content, &pages, "_/nav.md")?;

        let notfound_path = site_path.join("[404].md");
        let notfound_page = if notfound_path.exists() {
            Some(notfound_path)
        } else {
            None
        };

        // Pre-compile the root template
        let mut root_tera = Tera::default();
        root_tera
            .add_raw_template("root", ROOT_TEMPL)
            .expect("ROOT_TEMPL should always be valid Tera syntax");

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
            root_tera,
            highlight_css,
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

    SeoContext {
        description: description.clone(),
        author,
        canonical_url: canonical_url.clone(),
        og_title: frontmatter.title.clone(),
        og_description: description.clone(),
        og_url: canonical_url,
        og_type: "website".to_string(),
        og_image: image.clone(),
        og_site_name: site.title.clone(),
        twitter_card,
        twitter_title: frontmatter.title.clone(),
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

/// Result of scanning pages - separates static pages from dynamic definitions
pub struct ScanResult {
    pub static_pages: Vec<PageInfo>,
    pub dynamic_defs: Vec<DynamicPageDef>,
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

    /// Inject this dynamic context into a Tera Context
    pub fn inject_into(&self, context: &mut Context) {
        // Convert YamlValue to Tera Value
        let tera_value = yaml_to_tera_value(&self.param_value);
        context.insert(&self.param_name, &tera_value);
    }
}

/// Convert a YAML value to a Tera-compatible JSON value
fn yaml_to_tera_value(value: &YamlValue) -> serde_json::Value {
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
            serde_json::Value::Array(seq.iter().map(yaml_to_tera_value).collect())
        }
        YamlValue::Mapping(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        YamlValue::String(s) => s.clone(),
                        _ => return None,
                    };
                    Some((key, yaml_to_tera_value(v)))
                })
                .collect();
            serde_json::Value::Object(obj)
        }
        YamlValue::Tagged(tagged) => yaml_to_tera_value(&tagged.value),
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

/// Evaluate parameter values from frontmatter - either a direct array or a Tera expression
fn evaluate_param_values(
    param_name: &str,
    frontmatter: &YamlValue,
    source_path: &Path,
) -> Result<Vec<YamlValue>> {
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

        // Tera expression: page_no: "{{ range(end=5) }}" or page_no: "range(end=5)"
        YamlValue::String(expr) => {
            // Create a minimal Tera instance to evaluate the expression
            let mut tera = Tera::default();

            // Strip {{ }} wrapper if present (user can write either form)
            let clean_expr = expr
                .trim()
                .strip_prefix("{{")
                .and_then(|s| s.strip_suffix("}}"))
                .map(|s| s.trim())
                .unwrap_or(expr.trim());

            // Wrap expression to output JSON array
            let template = format!("{{% set result = {} %}}[{{% for item in result %}}{{{{ item }}}}{{% if not loop.last %}},{{% endif %}}{{% endfor %}}]", clean_expr);

            tera.add_raw_template("expr", &template).map_err(|e| {
                HugsError::DynamicExprEval {
                    file: source_path.display().to_string().into(),
                    param_name: param_name.into(),
                    expression: expr.clone(),
                    reason: e.to_string(),
                }
            })?;

            let context = Context::new();
            let output = tera.render("expr", &context).map_err(|e| {
                HugsError::DynamicExprEval {
                    file: source_path.display().to_string().into(),
                    param_name: param_name.into(),
                    expression: expr.clone(),
                    reason: e.to_string(),
                }
            })?;

            // Parse the JSON array output
            let values: Vec<serde_json::Value> =
                serde_json::from_str(&output).map_err(|e| HugsError::DynamicExprEval {
                    file: source_path.display().to_string().into(),
                    param_name: param_name.into(),
                    expression: expr.clone(),
                    reason: format!("Expression didn't produce a valid array: {}", e),
                })?;

            // Convert JSON values to YAML values
            Ok(values
                .into_iter()
                .map(|v| match v {
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            YamlValue::Number(i.into())
                        } else if let Some(f) = n.as_f64() {
                            YamlValue::Number(serde_yaml::Number::from(f))
                        } else {
                            YamlValue::String(n.to_string())
                        }
                    }
                    serde_json::Value::String(s) => YamlValue::String(s),
                    serde_json::Value::Bool(b) => YamlValue::Bool(b),
                    _ => YamlValue::String(v.to_string()),
                })
                .collect())
        }

        _ => Err(HugsError::DynamicParamParse {
            file: source_path.display().to_string().into(),
            param_name: param_name.into(),
            reason: "Parameter value must be an array or a Tera expression string".into(),
        }),
    }
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
    Dynamic(DynamicPageDef),
}

async fn scan_pages(site_path: &PathBuf) -> Result<ScanResult> {
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
                    warn!(
                        file = %relative_path.display(),
                        error = %e,
                        "I couldn't read this file, skipping it"
                    );
                    return None;
                }
            };

            let frontmatter = match markdown_frontmatter::parse::<YamlValue>(&content) {
                Ok((fm, _body)) => fm,
                Err(e) => {
                    warn!(
                        file = %relative_path.display(),
                        error = %e,
                        "I couldn't parse the frontmatter in this file, using empty metadata"
                    );
                    YamlValue::Mapping(serde_yaml::Mapping::new())
                }
            };

            // Check if this is a dynamic page
            if is_dynamic_page(&relative_path) {
                let filename = relative_path.file_name()?.to_str()?;
                let param_name = extract_param_name(filename)?;

                // Evaluate parameter values (can fail)
                let param_values = match evaluate_param_values(&param_name, &frontmatter, &relative_path) {
                    Ok(values) => values,
                    Err(e) => return Some(Err(e)),
                };

                Some(Ok(ParsedPage::Dynamic(DynamicPageDef {
                    param_name,
                    source_path: relative_path,
                    param_values,
                    frontmatter,
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
    let mut dynamic_defs = Vec::new();

    while let Some(result) = join_set.join_next().await {
        if let Ok(Some(parsed_result)) = result {
            match parsed_result? {
                ParsedPage::Static(page_info) => static_pages.push(page_info),
                ParsedPage::Dynamic(def) => dynamic_defs.push(def),
            }
        }
    }

    Ok(ScanResult {
        static_pages,
        dynamic_defs,
    })
}

#[derive(Serialize)]
pub struct PageContent<'a> {
    pub title: &'a str,
    pub header: &'a str,
    pub footer: &'a str,
    pub nav: &'a str,
    pub content: &'a str,
    pub path_class: &'a str,
    pub base: &'a str,
    pub dev_script: &'a str,
    pub seo: SeoContext,
    pub syntax_highlighting_enabled: bool,
}

/// Resolve a URL path to a document, returning the frontmatter, HTML content, and file path.
///
/// Returns:
/// - `Ok(Some(...))` if the page was found and rendered successfully
/// - `Ok(None)` if no page exists at this path (404)
/// - `Err(...)` if an error occurred while processing the page
pub async fn resolve_path_to_doc(
    path: &str,
    app_data: &AppData,
) -> Result<Option<(ContentFrontmatter, String, PathBuf)>> {
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

    let doc_content_tera = tokio::fs::read_to_string(&resolvable_path)
        .await
        .with_file_read(&resolvable_path)?;

    let path_class = convert_path_to_class(&resolvable_path, app_data)?;

    let initial_page_content = PageContent {
        title: "",
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: "",
        path_class: &path_class,
        base: "/",
        dev_script: "",
        seo: SeoContext::default(),
        syntax_highlighting_enabled: false,
    };

    let context =
        Context::from_serialize(&initial_page_content).map_err(|e| HugsError::TemplateContext {
            reason: e.to_string(),
        })?;

    let doc_content = render_template(&doc_content_tera, &context, &app_data.pages, None)
        .map_err(|e| HugsError::template_render(&resolvable_path, &doc_content_tera, e))?;

    let (frontmatter, body) =
        markdown_frontmatter::parse::<ContentFrontmatter>(&doc_content).map_err(|e| {
            // The frontmatter parsing error from the library doesn't give us good location info,
            // but we can try to extract what we can
            HugsError::FrontmatterParse {
                file: relative_path_str.clone().into(),
                src: miette::NamedSource::new(relative_path_str.clone(), doc_content.clone()),
                span: miette::SourceSpan::from((0_usize, 1_usize)),
                reason: format!(
                    "I couldn't parse the frontmatter. Make sure you have a valid `title` field. Error: {}",
                    e
                ),
            }
        })?;

    let doc_html = markdown_to_html(body, &app_data.config.build.syntax_highlighting)
        .map_err(|reason| HugsError::MarkdownParse {
            file: relative_path_str.into(),
            reason,
        })?;

    Ok(Some((frontmatter, doc_html, resolvable_path)))
}

/// Resolve a dynamic page from its source file path with dynamic context.
///
/// This is used for dynamic pages like `[slug].md` where we need to inject
/// the parameter value into the template context.
pub async fn resolve_dynamic_doc(
    source_file_path: &str,
    dynamic_ctx: &DynamicContext,
    app_data: &AppData,
) -> Result<(ContentFrontmatter, String, PathBuf)> {
    let resolvable_path = app_data.site_path.join(source_file_path);

    let relative_path_str = source_file_path.to_string();

    let doc_content_tera = tokio::fs::read_to_string(&resolvable_path)
        .await
        .with_file_read(&resolvable_path)?;

    // For dynamic pages, use the param value in the path class (not the [param] placeholder)
    let value_str = yaml_value_to_string(&dynamic_ctx.param_value);
    let path_class = source_file_path
        .strip_suffix(".md")
        .unwrap_or(source_file_path)
        .replace(&format!("[{}]", dynamic_ctx.param_name), &value_str)
        .replace('/', " ");

    let initial_page_content = PageContent {
        title: "",
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: "",
        path_class: &path_class,
        base: "/",
        dev_script: "",
        seo: SeoContext::default(),
        syntax_highlighting_enabled: false,
    };

    // Create context and inject the dynamic parameter
    let mut context =
        Context::from_serialize(&initial_page_content).map_err(|e| HugsError::TemplateContext {
            reason: e.to_string(),
        })?;

    // Inject the dynamic parameter (e.g., `slug` = "hello")
    dynamic_ctx.inject_into(&mut context);

    let doc_content = render_template(&doc_content_tera, &context, &app_data.pages, None)
        .map_err(|e| HugsError::template_render(&resolvable_path, &doc_content_tera, e))?;

    let (frontmatter, body) =
        markdown_frontmatter::parse::<ContentFrontmatter>(&doc_content).map_err(|e| {
            HugsError::FrontmatterParse {
                file: relative_path_str.clone().into(),
                src: miette::NamedSource::new(relative_path_str.clone(), doc_content.clone()),
                span: miette::SourceSpan::from((0_usize, 1_usize)),
                reason: format!(
                    "I couldn't parse the frontmatter. Make sure you have a valid `title` field. Error: {}",
                    e
                ),
            }
        })?;

    let doc_html = markdown_to_html(body, &app_data.config.build.syntax_highlighting)
        .map_err(|reason| HugsError::MarkdownParse {
            file: relative_path_str.into(),
            reason,
        })?;

    Ok((frontmatter, doc_html, resolvable_path))
}

pub async fn render_notfound_page(app_data: &AppData, dev_script: &str) -> Option<String> {
    let notfound_path = app_data.notfound_page.as_ref()?;

    let doc_content_tera = tokio::fs::read_to_string(notfound_path).await.ok()?;

    let initial_page_content = PageContent {
        title: "",
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: "",
        path_class: "notfound",
        base: "/",
        dev_script: "",
        seo: SeoContext::default(),
        syntax_highlighting_enabled: false,
    };

    let context = Context::from_serialize(&initial_page_content).ok()?;
    let doc_content = render_template(&doc_content_tera, &context, &app_data.pages, None).ok()?;

    let (frontmatter, body) = markdown_frontmatter::parse::<ContentFrontmatter>(&doc_content).ok()?;

    let doc_html = markdown_to_html(body, &app_data.config.build.syntax_highlighting).ok()?;

    let seo = build_seo_context(&frontmatter, "/404", &app_data.config.site);
    let content = PageContent {
        title: &frontmatter.title,
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: &doc_html,
        path_class: "notfound",
        base: "/",
        dev_script,
        seo,
        syntax_highlighting_enabled: app_data.config.build.syntax_highlighting.enabled,
    };

    let context = Context::from_serialize(&content).ok()?;
    let cache_bust = app_data.cache_bust_function();
    let html_out = render_root_template(app_data, &context, &cache_bust).ok()?;

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

    render_page_html_internal(frontmatter, doc_html, &page_url, &path_class, &base, app_data, dev_script)
}

/// Render a dynamic page to HTML with explicit URL (for proper SEO and path_class)
pub fn render_dynamic_page_html(
    frontmatter: &ContentFrontmatter,
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

    render_page_html_internal(frontmatter, doc_html, page_url, &path_class, &base, app_data, dev_script)
}

/// Internal helper for rendering page HTML
fn render_page_html_internal(
    frontmatter: &ContentFrontmatter,
    doc_html: &str,
    page_url: &str,
    path_class: &str,
    base: &str,
    app_data: &AppData,
    dev_script: &str,
) -> Result<String> {
    let seo = build_seo_context(frontmatter, page_url, &app_data.config.site);

    let content = PageContent {
        title: &frontmatter.title,
        header: &app_data.header_html,
        footer: &app_data.footer_html,
        nav: &app_data.nav_html,
        content: doc_html,
        path_class,
        base,
        dev_script,
        seo,
        syntax_highlighting_enabled: app_data.config.build.syntax_highlighting.enabled,
    };

    let context =
        Context::from_serialize(&content).map_err(|e| HugsError::TemplateContext {
            reason: e.to_string(),
        })?;

    let cache_bust = app_data.cache_bust_function();
    render_root_template(app_data, &context, &cache_bust)
        .map_err(|e| HugsError::template_render_named("root.tera", ROOT_TEMPL, &e))
}
