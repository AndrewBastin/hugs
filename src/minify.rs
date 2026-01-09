use minify_html::{minify, Cfg};

/// Configuration for minification
#[derive(Debug, Clone, Copy)]
pub struct MinifyConfig {
    pub enabled: bool,
}

impl MinifyConfig {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

/// Minify HTML content
pub fn minify_html_content(html: &str, config: &MinifyConfig) -> String {
    if !config.enabled {
        return html.to_string();
    }

    let cfg = Cfg {
        minify_css: true,
        minify_js: false,
        ..Cfg::default()
    };

    let minified = minify(html.as_bytes(), &cfg);
    String::from_utf8(minified).unwrap_or_else(|_| html.to_string())
}

/// Minify CSS content
pub fn minify_css_content(css: &str, config: &MinifyConfig) -> String {
    if !config.enabled {
        return css.to_string();
    }

    // Wrap CSS in a style tag to use minify-html's CSS minification
    let wrapped = format!("<style>{}</style>", css);
    let cfg = Cfg {
        minify_css: true,
        ..Cfg::default()
    };

    let minified = minify(wrapped.as_bytes(), &cfg);
    let result = String::from_utf8(minified).unwrap_or_else(|_| css.to_string());

    // Extract CSS from <style>...</style>
    result
        .strip_prefix("<style>")
        .and_then(|s| s.strip_suffix("</style>"))
        .unwrap_or(&result)
        .to_string()
}
