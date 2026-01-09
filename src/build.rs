use std::path::PathBuf;
use std::sync::Arc;

use tokio::task::JoinSet;
use tracing::info;
use walkdir::WalkDir;

use crate::error::{HugsError, Result};
use crate::feed::{collect_feed_items, generate_atom, generate_rss};
use crate::minify::{minify_css_content, minify_html_content, MinifyConfig};
use crate::run::{render_notfound_page, render_page_html, render_dynamic_page_html, resolve_path_to_doc, resolve_dynamic_doc, DynamicContext, AppData};
use crate::sitemap::generate_sitemap;

/// Collected warnings during the build process
#[derive(Default)]
struct BuildWarnings {
    warnings: Vec<HugsError>,
}

impl BuildWarnings {
    fn add(&mut self, error: HugsError) {
        self.warnings.push(error);
    }

    /// Display all collected warnings using miette's fancy formatting
    fn display(&self) {
        if self.warnings.is_empty() {
            return;
        }

        eprintln!();
        let warning_word = if self.warnings.len() == 1 {
            "warning"
        } else {
            "warnings"
        };
        eprintln!(
            "\x1b[33;1m⚠ Build completed with {} {}\x1b[0m\n",
            self.warnings.len(),
            warning_word
        );

        for warning in &self.warnings {
            let report = miette::Report::new(warning.clone());
            eprintln!("{:?}", report);
        }
    }
}

pub async fn run_build(site_path: PathBuf, output_path: PathBuf) -> Result<()> {
    info!(
        site = %site_path.display(),
        output = %output_path.display(),
        "Building site"
    );

    let mut warnings = BuildWarnings::default();

    // Load site data (wrapped in Arc for parallel rendering)
    let app_data = Arc::new(AppData::load(site_path).await?);
    let minify_config = MinifyConfig::new(app_data.config.build.minify);

    // Clean/create output directory
    clean_output_directory(&output_path).await?;

    // Render all pages (in parallel)
    let page_count =
        render_all_pages(Arc::clone(&app_data), output_path.clone(), minify_config).await?;

    // Render 404 page if it exists
    render_404_page(&app_data, &output_path, &minify_config).await?;

    // Generate feeds
    let feed_count = generate_feeds(&app_data, &output_path, &mut warnings).await?;

    // Generate sitemap
    let sitemap_generated = generate_sitemap_file(&app_data, &output_path, &mut warnings).await?;

    // Copy static assets
    let asset_count = copy_static_assets(&app_data.site_path, &output_path).await?;

    // Write cache-busted assets (from cache_bust() template function)
    write_cache_busted_assets(&app_data, &output_path, &minify_config).await?;

    // Write theme.css (only if not cache-busted)
    write_theme_css(&app_data, &output_path, &minify_config).await?;

    let sitemap_msg = if sitemap_generated { ", sitemap" } else { "" };
    info!(
        pages = page_count,
        feeds = feed_count,
        assets = asset_count,
        "Build complete! {} pages, {} feeds{}, {} assets",
        page_count,
        feed_count,
        sitemap_msg,
        asset_count
    );

    // Display any collected warnings with fancy formatting
    warnings.display();

    Ok(())
}

async fn clean_output_directory(output_path: &PathBuf) -> Result<()> {
    if output_path.exists() {
        info!("Cleaning output directory...");
        tokio::fs::remove_dir_all(output_path)
            .await
            .map_err(|e| HugsError::CreateDir {
                path: output_path.into(),
                cause: e,
            })?;
    }
    tokio::fs::create_dir_all(output_path)
        .await
        .map_err(|e| HugsError::CreateDir {
            path: output_path.into(),
            cause: e,
        })?;
    Ok(())
}

async fn render_all_pages(
    app_data: Arc<AppData>,
    output_path: PathBuf,
    minify_config: MinifyConfig,
) -> Result<usize> {
    let page_count = app_data.pages.len();
    info!(count = page_count, "Rendering pages...");

    let mut join_set: JoinSet<Result<()>> = JoinSet::new();

    for page_info in app_data.pages.iter() {
        let app_data = Arc::clone(&app_data);
        let output_path = output_path.clone();
        let url = page_info.url.clone();
        let file_path = page_info.file_path.clone();
        // Check if this is a dynamic page and extract context
        let dynamic_ctx = DynamicContext::from_page_info(page_info);

        join_set.spawn(async move {
            // Resolve the page and render - use appropriate method for dynamic vs static pages
            let html_out = if let Some(ctx) = &dynamic_ctx {
                // Dynamic page: resolve from source file with context
                let (frontmatter, doc_html, _resolvable_path) =
                    resolve_dynamic_doc(&file_path, ctx, &app_data).await?;
                // Use the resolved URL (e.g., /docs/2) for proper SEO
                render_dynamic_page_html(&frontmatter, &doc_html, &url, &app_data, "")?
            } else {
                // Static page: resolve from URL path
                let request_path = url.trim_start_matches('/');
                let (frontmatter, doc_html, resolvable_path) =
                    resolve_path_to_doc(request_path, &app_data)
                        .await?
                        .ok_or_else(|| HugsError::PageResolve {
                            url: url.clone().into(),
                            file_path: file_path.clone().into(),
                        })?;
                render_page_html(&frontmatter, &doc_html, &resolvable_path, &app_data, "")?
            };

            // Apply minification if enabled
            let final_html = minify_html_content(&html_out, &minify_config);

            // Write to output
            let output_file = url_to_output_path(&url, &output_path);
            if let Some(parent) = output_file.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| HugsError::CreateDir {
                        path: parent.into(),
                        cause: e,
                    })?;
            }

            info!(
                source = %file_path,
                output = %output_file.display(),
                "Rendered page"
            );
            tokio::fs::write(&output_file, final_html)
                .await
                .map_err(|e| HugsError::FileWrite {
                    path: (&output_file).into(),
                    cause: e,
                })?;

            Ok(())
        });
    }

    // Wait for all tasks to complete
    while let Some(result) = join_set.join_next().await {
        // Propagate both JoinError (task panic) and render errors
        result.map_err(|e| HugsError::TaskJoin {
            reason: e.to_string(),
        })??;
    }

    Ok(page_count)
}

fn url_to_output_path(url: &str, output_path: &PathBuf) -> PathBuf {
    if url == "/" {
        output_path.join("index.html")
    } else if url.ends_with('/') {
        // /blog/ -> dist/blog/index.html
        let dir = url.trim_matches('/');
        output_path.join(dir).join("index.html")
    } else {
        // /about -> dist/about/index.html
        let dir = url.trim_start_matches('/');
        output_path.join(dir).join("index.html")
    }
}

async fn render_404_page(
    app_data: &AppData,
    output_path: &PathBuf,
    minify_config: &MinifyConfig,
) -> Result<()> {
    if let Some(html) = render_notfound_page(app_data, "").await {
        let final_html = minify_html_content(&html, minify_config);
        let output_file = output_path.join("404.html");
        info!(output = %output_file.display(), "Rendered 404 page");
        tokio::fs::write(&output_file, final_html)
            .await
            .map_err(|e| HugsError::FileWrite {
                path: (&output_file).into(),
                cause: e,
            })?;
    }
    Ok(())
}

async fn copy_static_assets(site_path: &PathBuf, output_path: &PathBuf) -> Result<usize> {
    let mut count = 0;

    for entry in WalkDir::new(site_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let relative = path.strip_prefix(site_path).unwrap_or(path);

        // Skip _ directory
        if relative.starts_with("_") {
            continue;
        }

        // Skip markdown files (they're rendered as pages)
        if path.extension().is_some_and(|ext| ext == "md") {
            continue;
        }

        // Copy to output
        let output_file = output_path.join(relative);
        if let Some(parent) = output_file.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| HugsError::CreateDir {
                    path: parent.into(),
                    cause: e,
                })?;
        }

        tokio::fs::copy(path, &output_file)
            .await
            .map_err(|e| HugsError::CopyFile {
                src: path.into(),
                dest: (&output_file).into(),
                cause: e,
            })?;
        count += 1;
    }

    if count > 0 {
        info!(count, "Copied static assets");
    }

    Ok(count)
}

async fn write_theme_css(
    app_data: &AppData,
    output_path: &PathBuf,
    minify_config: &MinifyConfig,
) -> Result<()> {
    // Skip if theme.css was cache-busted (it's already written with hashed name)
    let entries = app_data.cache_bust_registry.entries();
    if entries.contains_key("/theme.css") {
        return Ok(());
    }

    info!("Writing theme.css");
    let css_path = output_path.join("theme.css");
    let final_css = minify_css_content(&app_data.theme_css, minify_config);
    tokio::fs::write(&css_path, final_css)
        .await
        .map_err(|e| HugsError::FileWrite {
            path: (&css_path).into(),
            cause: e,
        })?;
    Ok(())
}

async fn write_cache_busted_assets(
    app_data: &AppData,
    output_path: &PathBuf,
    minify_config: &MinifyConfig,
) -> Result<()> {
    let entries = app_data.cache_bust_registry.entries();

    if entries.is_empty() {
        return Ok(());
    }

    for (original_path, hashed_path) in entries {
        let hashed_filename = hashed_path.trim_start_matches('/');

        if original_path == "/theme.css" {
            // theme.css is pre-loaded in app_data
            let dest = output_path.join(hashed_filename);
            info!(
                original = %original_path,
                hashed = %hashed_path,
                "Writing cache-busted asset"
            );
            let final_css = minify_css_content(&app_data.theme_css, minify_config);
            tokio::fs::write(&dest, final_css)
                .await
                .map_err(|e| HugsError::FileWrite {
                    path: (&dest).into(),
                    cause: e,
                })?;
        } else if original_path == "/highlight.css" {
            // highlight.css is pre-generated in app_data
            let dest = output_path.join(hashed_filename);
            info!(
                original = %original_path,
                hashed = %hashed_path,
                "Writing cache-busted asset"
            );
            let final_css = minify_css_content(&app_data.highlight_css, minify_config);
            tokio::fs::write(&dest, final_css)
                .await
                .map_err(|e| HugsError::FileWrite {
                    path: (&dest).into(),
                    cause: e,
                })?;
        } else {
            // Regular files: read from site_path
            let src = app_data
                .site_path
                .join(original_path.trim_start_matches('/'));
            let dest = output_path.join(hashed_filename);

            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| HugsError::CreateDir {
                        path: parent.into(),
                        cause: e,
                    })?;
            }

            info!(
                original = %original_path,
                hashed = %hashed_path,
                "Writing cache-busted asset"
            );
            tokio::fs::copy(&src, &dest)
                .await
                .map_err(|e| HugsError::CopyFile {
                    src: (&src).into(),
                    dest: (&dest).into(),
                    cause: e,
                })?;
        }
    }

    Ok(())
}

async fn generate_feeds(
    app_data: &AppData,
    output_path: &PathBuf,
    warnings: &mut BuildWarnings,
) -> Result<usize> {
    if app_data.config.feeds.is_empty() {
        return Ok(0);
    }

    info!(
        count = app_data.config.feeds.len(),
        "Generating feed(s)..."
    );

    let mut count = 0;

    for feed_config in &app_data.config.feeds {
        let items = collect_feed_items(&app_data.pages, feed_config, &app_data.config.site);

        // Generate RSS if configured
        if let Some(rss_filename) = &feed_config.output_rss {
            match generate_rss(&items, feed_config, &app_data.config.site) {
                Ok(rss_xml) => {
                    let rss_path = output_path.join(rss_filename);
                    info!(
                        file = %rss_filename,
                        items = items.len(),
                        "Generated RSS feed"
                    );
                    tokio::fs::write(&rss_path, rss_xml)
                        .await
                        .map_err(|e| HugsError::FileWrite {
                            path: (&rss_path).into(),
                            cause: e,
                        })?;
                    count += 1;
                }
                Err(e) => {
                    warnings.add(e);
                }
            }
        }

        // Generate Atom if configured
        if let Some(atom_filename) = &feed_config.output_atom {
            match generate_atom(&items, feed_config, &app_data.config.site) {
                Ok(atom_xml) => {
                    let atom_path = output_path.join(atom_filename);
                    info!(
                        file = %atom_filename,
                        items = items.len(),
                        "Generated Atom feed"
                    );
                    tokio::fs::write(&atom_path, atom_xml)
                        .await
                        .map_err(|e| HugsError::FileWrite {
                            path: (&atom_path).into(),
                            cause: e,
                        })?;
                    count += 1;
                }
                Err(e) => {
                    warnings.add(e);
                }
            }
        }
    }

    Ok(count)
}

async fn generate_sitemap_file(
    app_data: &AppData,
    output_path: &PathBuf,
    warnings: &mut BuildWarnings,
) -> Result<bool> {
    // Only generate if site.url is configured
    if app_data.config.site.url.is_none() {
        return Ok(false);
    }

    info!("Generating sitemap.xml...");

    match generate_sitemap(&app_data.pages, &app_data.config.site) {
        Ok(sitemap_xml) => {
            let sitemap_path = output_path.join("sitemap.xml");
            info!(urls = app_data.pages.len(), "Generated sitemap.xml");
            tokio::fs::write(&sitemap_path, sitemap_xml)
                .await
                .map_err(|e| HugsError::FileWrite {
                    path: (&sitemap_path).into(),
                    cause: e,
                })?;
            Ok(true)
        }
        Err(e) => {
            warnings.add(e);
            Ok(false)
        }
    }
}
