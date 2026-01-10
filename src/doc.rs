use std::path::PathBuf;
use std::sync::Arc;

use actix_web::{App, HttpResponse, HttpServer, get, http::header::ContentType, web};
use include_dir::{Dir, include_dir};
use owo_colors::OwoColorize;
use tokio::fs;
use tracing::{info, warn};

use crate::error::{HugsError, Result, StyledPath, StyledNum};
use crate::minify::{minify_css_content, minify_html_content, MinifyConfig};
use crate::run::{
    render_notfound_page, render_page_html, resolve_path_to_doc,
    try_serve_static_file, AppData,
};
use crate::sitemap::generate_sitemap;

/// The tutorial site directory embedded at compile time
static DOCS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/tutorial-site");

/// Maximum number of port retry attempts before giving up
const MAX_PORT_RETRIES: u16 = 50;

pub struct DocAppState {
    pub app_data: AppData,
    pub minify_config: MinifyConfig,
}

#[get("/theme.css")]
async fn theme(state: web::Data<Arc<DocAppState>>) -> HttpResponse {
    let css = minify_css_content(&state.app_data.theme_css, &state.minify_config);
    HttpResponse::Ok()
        .content_type(ContentType(mime_guess::mime::TEXT_CSS_UTF_8))
        .body(css)
}

#[get("/theme.{hash}.css")]
async fn theme_hashed(state: web::Data<Arc<DocAppState>>) -> HttpResponse {
    let css = minify_css_content(&state.app_data.theme_css, &state.minify_config);
    HttpResponse::Ok()
        .content_type(ContentType(mime_guess::mime::TEXT_CSS_UTF_8))
        .body(css)
}

#[get("/sitemap.xml")]
async fn sitemap(state: web::Data<Arc<DocAppState>>) -> HttpResponse {
    match generate_sitemap(&state.app_data.pages, &state.app_data.config.site) {
        Ok(xml) => HttpResponse::Ok()
            .content_type(ContentType::xml())
            .body(xml),
        Err(_) => HttpResponse::InternalServerError()
            .body("Sitemap generation failed"),
    }
}

#[get("/{tail:.*}")]
async fn page(path: web::Path<String>, state: web::Data<Arc<DocAppState>>) -> HttpResponse {
    let path_str = path.trim_end_matches('/');

    if let Some(response) = try_serve_static_file(path_str, &state.app_data).await {
        return response;
    }

    match resolve_path_to_doc(path_str, &state.app_data).await {
        Ok(Some((frontmatter, doc_html, resolvable_path, frontmatter_json))) => {
            match render_page_html(
                &frontmatter,
                &frontmatter_json,
                &doc_html,
                &resolvable_path,
                &state.app_data,
                "", // No live reload script for doc server
            ) {
                Ok(html_out) => {
                    let final_html = minify_html_content(&html_out, &state.minify_config);
                    HttpResponse::Ok()
                        .content_type(ContentType::html())
                        .body(final_html)
                }
                Err(_) => HttpResponse::InternalServerError()
                    .body("Render error"),
            }
        }
        Ok(None) => {
            if let Some(html) = render_notfound_page(&state.app_data, "").await {
                let final_html = minify_html_content(&html, &state.minify_config);
                HttpResponse::NotFound()
                    .content_type(ContentType::html())
                    .body(final_html)
            } else {
                HttpResponse::NotFound()
                    .body("Not Found")
            }
        }
        Err(_) => HttpResponse::InternalServerError()
            .body("Error processing page"),
    }
}

/// Run the documentation server
pub async fn run_doc_server(port: Option<u16>, no_open: bool) -> Result<()> {
    info!("Starting documentation server...");

    // Extract docs to temp directory
    let temp_dir = extract_docs_to_temp().await?;
    let docs_path = temp_dir.path().to_path_buf();

    info!(path = %docs_path.display(), "Extracted documentation");

    // Load site data
    let app_data = AppData::load(docs_path).await?;
    let minify_config = MinifyConfig::new(app_data.config.build.minify);

    let state = Arc::new(DocAppState {
        app_data,
        minify_config,
    });

    // Find available port
    let default_port = port.unwrap_or(8888);
    let port_explicit = port.is_some();
    let (server, actual_port) = try_bind_server(Arc::clone(&state), default_port, port_explicit)?;

    let url = format!("http://127.0.0.1:{}", actual_port);

    println!();
    println!(
        "  {} Documentation server running at {}",
        "~".cyan().bold(),
        url.cyan().bold()
    );
    println!();

    // Open browser (unless --no-open)
    if !no_open {
        let url_clone = url.clone();
        tokio::spawn(async move {
            // Small delay to ensure server is ready
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if let Err(e) = open::that(&url_clone) {
                warn!("Couldn't open browser: {}", e);
                println!("  Open {} in your browser", url_clone.cyan());
            }
        });
    }

    // Run server (temp_dir stays alive while server runs)
    server
        .await
        .map_err(|e| HugsError::ServerRuntime { cause: e })?;

    // temp_dir dropped here, cleaning up
    drop(temp_dir);
    Ok(())
}

/// Extract embedded docs directory to a temporary directory
async fn extract_docs_to_temp() -> Result<tempfile::TempDir> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| HugsError::DocTempDir { cause: e })?;

    extract_dir(&DOCS_DIR, &temp_dir.path().to_path_buf()).await?;

    Ok(temp_dir)
}

/// Recursively extract an embedded directory to the filesystem
async fn extract_dir(dir: &Dir<'_>, target: &PathBuf) -> Result<()> {
    fs::create_dir_all(target)
        .await
        .map_err(|e| HugsError::CreateDir {
            path: StyledPath::from(target),
            cause: e,
        })?;

    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::Dir(subdir) => {
                let subdir_path = target.join(subdir.path().file_name().unwrap());
                Box::pin(extract_dir(subdir, &subdir_path)).await?;
            }
            include_dir::DirEntry::File(file) => {
                let file_path = target.join(file.path().file_name().unwrap());
                fs::write(&file_path, file.contents())
                    .await
                    .map_err(|e| HugsError::FileWrite {
                        path: StyledPath::from(&file_path),
                        cause: e,
                    })?;
            }
        }
    }

    Ok(())
}

/// Attempt to bind to a port, retrying with incrementing ports if port was not explicitly specified
fn try_bind_server(
    state: Arc<DocAppState>,
    port: u16,
    port_explicit: bool,
) -> Result<(actix_web::dev::Server, u16)> {
    if port_explicit {
        let state_for_server = Arc::clone(&state);
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(Arc::clone(&state_for_server)))
                .service(theme)
                .service(theme_hashed)
                .service(sitemap)
                .service(page)
        })
        .bind(("127.0.0.1", port))
        .map_err(|e| HugsError::PortBind {
            port: StyledNum(port),
            src: miette::NamedSource::new(
                "command".to_string(),
                format!("hugs doc --port {}", port),
            ),
            span: miette::SourceSpan::new(
                (17).into(), // position of port number
                port.to_string().len().into(),
            ),
            help_text: format!(
                "Port {} is already in use. Try a different port with: {}",
                port.bold(),
                format!("hugs doc --port {}", port.saturating_add(1)).cyan()
            ),
            cause: e,
        })?;

        Ok((server.run(), port))
    } else {
        for attempt in 0..MAX_PORT_RETRIES {
            let try_port = match port.checked_add(attempt) {
                Some(p) => p,
                None => break,
            };

            let state_for_server = Arc::clone(&state);
            match HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(Arc::clone(&state_for_server)))
                    .service(theme)
                    .service(theme_hashed)
                    .service(sitemap)
                    .service(page)
            })
            .bind(("127.0.0.1", try_port))
            {
                Ok(server) => {
                    return Ok((server.run(), try_port));
                }
                Err(_) => {
                    continue;
                }
            }
        }

        let end_port = port.saturating_add(MAX_PORT_RETRIES - 1);
        Err(HugsError::NoAvailablePort {
            start_port: port.into(),
            end_port: end_port.into(),
        })
    }
}
