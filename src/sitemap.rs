use serde::Serialize;
use tera::{Context, Tera};

use crate::config::SiteMetadata;
use crate::error::{HugsError, Result};
use crate::feed::extract_date_from_frontmatter;
use crate::run::PageInfo;

const SITEMAP_TEMPLATE: &str = include_str!("templates/sitemap.tera");

#[derive(Serialize)]
struct SitemapEntry {
    loc: String,
    lastmod: Option<String>,
}

/// Generate a sitemap.xml for all pages
pub fn generate_sitemap(pages: &[PageInfo], site_metadata: &SiteMetadata) -> Result<String> {
    let base_url = site_metadata
        .url
        .as_ref()
        .ok_or(HugsError::SitemapMissingUrl)?;
    let base_url = base_url.trim_end_matches('/');

    let entries: Vec<SitemapEntry> = pages
        .iter()
        .map(|page| {
            let url_with_slash = if page.url.ends_with('/') {
                page.url.clone()
            } else {
                format!("{}/", page.url)
            };

            let lastmod = extract_date_from_frontmatter(&page.frontmatter)
                .map(|dt| dt.format("%Y-%m-%d").to_string());

            SitemapEntry {
                loc: format!("{}{}", base_url, url_with_slash),
                lastmod,
            }
        })
        .collect();

    let mut tera = Tera::default();
    tera.add_raw_template("sitemap", SITEMAP_TEMPLATE)
        .map_err(|e| HugsError::SitemapTemplate {
            reason: e.to_string(),
        })?;

    let mut context = Context::new();
    context.insert("entries", &entries);

    tera.render("sitemap", &context)
        .map_err(|e| HugsError::SitemapTemplate {
            reason: e.to_string(),
        })
}
