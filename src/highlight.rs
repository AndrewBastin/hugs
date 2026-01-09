//! Syntax highlighting for code blocks using giallo.

use std::sync::OnceLock;

use giallo::{HighlightOptions, HtmlRenderer, Registry, RenderOptions, ThemeVariant};
use regex::Regex;

/// Global registry - loaded once at startup
static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Regex for finding code blocks in HTML
static CODE_BLOCK_RE: OnceLock<Regex> = OnceLock::new();

/// Initialize the syntax highlighting registry.
/// This should be called once at application startup.
pub fn init_registry() {
    REGISTRY.get_or_init(|| {
        let mut registry = Registry::builtin().expect("Failed to load syntax highlighting registry");
        registry.link_grammars();
        registry
    });
    CODE_BLOCK_RE.get_or_init(|| {
        // Match <pre><code class="language-X">...</code></pre>
        // The (?s) flag makes . match newlines
        Regex::new(r#"(?s)<pre><code class="language-([^"]+)">(.+?)</code></pre>"#)
            .expect("Invalid regex pattern")
    });
}

/// Get the registry, panics if not initialized
fn registry() -> &'static Registry {
    REGISTRY
        .get()
        .expect("Syntax highlighting registry not initialized. Call init_registry() first.")
}

/// Get the code block regex
fn code_block_regex() -> &'static Regex {
    CODE_BLOCK_RE.get().expect("Code block regex not initialized")
}

/// HTML-decode common entities that markdown encoders produce
fn html_decode(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Highlight a single code block
fn highlight_code(code: &str, lang: &str, theme: &str) -> Option<String> {
    let registry = registry();

    let options = HighlightOptions::new(lang, ThemeVariant::Single(theme));

    // Try to highlight with the specified language
    let highlighted = registry.highlight(code, &options).ok()?;
    let renderer = HtmlRenderer::default();
    let render_options = RenderOptions::default();
    Some(renderer.render(&highlighted, &render_options))
}

/// Process HTML and highlight all code blocks.
/// Returns the HTML with code blocks syntax-highlighted.
pub fn highlight_code_blocks(html: &str, theme: &str) -> String {
    let re = code_block_regex();

    re.replace_all(html, |caps: &regex::Captures| {
        let lang = &caps[1];
        let code = html_decode(&caps[2]);

        match highlight_code(&code, lang, theme) {
            Some(highlighted) => highlighted,
            None => caps[0].to_string(), // Fall back to original on error
        }
    })
    .to_string()
}

/// Generate CSS for syntax highlighting theme.
pub fn generate_theme_css(theme: &str) -> String {
    let registry = registry();
    // The second argument is the CSS class prefix
    registry.generate_css(theme, "").unwrap_or_default()
}
