use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{HugsError, Result};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SiteConfig {
    #[serde(default)]
    pub site: SiteMetadata,
    #[serde(default)]
    pub feeds: Vec<FeedConfig>,
    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BuildConfig {
    /// Enable HTML and CSS minification
    #[serde(default = "default_true")]
    pub minify: bool,

    /// Syntax highlighting configuration
    #[serde(default)]
    pub syntax_highlighting: SyntaxHighlightConfig,

    /// Reading speed in words per minute for readtime calculation
    #[serde(default = "default_reading_speed")]
    pub reading_speed: u32,
}

fn default_reading_speed() -> u32 {
    200
}

fn default_true() -> bool {
    true
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            minify: true,
            syntax_highlighting: SyntaxHighlightConfig::default(),
            reading_speed: default_reading_speed(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyntaxHighlightConfig {
    /// Enable syntax highlighting for code blocks
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Theme name for syntax highlighting
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "one-dark-pro".to_string()
}

impl Default for SyntaxHighlightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SiteMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub author: Option<String>,
    #[serde(default = "default_language")]
    pub language: String,
    pub twitter_handle: Option<String>,
    pub default_image: Option<String>,
    /// Template for page titles, e.g. "{{ title }} | {{ site.title }}"
    pub title_template: Option<String>,
}

fn default_language() -> String {
    "en-us".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedConfig {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub source: String,
    pub output_rss: Option<String>,
    pub output_atom: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

impl SiteConfig {
    pub async fn load(site_path: &PathBuf) -> Result<Self> {
        let config_path = site_path.join("config.toml");

        if !config_path.exists() {
            return Ok(SiteConfig::default());
        }

        let content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| HugsError::ConfigRead {
                path: (&config_path).into(),
                cause: e,
            })?;

        toml::from_str(&content).map_err(|e| HugsError::config_parse(&config_path, &content, e))
    }
}
