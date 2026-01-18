use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, get, http::header::ContentType, web};
use actix_web_actors::ws;
use miette::Diagnostic;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind, event::ModifyKind};
use owo_colors::OwoColorize;
use thiserror::Error;
use tokio::sync::{RwLock, broadcast};

use crate::console;

use crate::error::{render_error_html, HugsError, Result};
use crate::minify::{minify_css_content, minify_html_content, MinifyConfig};
use crate::run::{
    render_notfound_page, render_page_html, render_dynamic_page_html, resolve_path_to_doc,
    resolve_dynamic_doc, try_serve_static_file, AppData, DynamicContext,
};
use crate::sitemap::generate_sitemap;

/// Maximum number of port retry attempts before giving up
const MAX_PORT_RETRIES: u16 = 50;

/// The default port number assigned for the dev server if no port is explicitly given
const DEFAULT_PORT: u16 = 8080;

/// A port number that displays with bold cyan highlighting
#[derive(Debug, Clone, Copy)]
struct StyledPort(u16);

impl std::fmt::Display for StyledPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.cyan().bold())
    }
}

/// Warning for port change during dev server startup
#[derive(Error, Diagnostic, Debug, Clone)]
#[error("I couldn't use the default port {DEFAULT_PORT}, so I'm using port {actual_port} instead")]
#[diagnostic(code(hugs::dev::port_changed), severity(warning))]
struct PortChangedWarning {
    actual_port: StyledPort,
    #[help]
    help_text: String,
}

impl PortChangedWarning {
    fn new(actual_port: u16) -> Self {
        Self {
            actual_port: StyledPort(actual_port),
            help_text: format!(
                "The default port was already in use. If you'd like me to fail instead of retrying, specify a port explicitly with {}",
                "--port".cyan().bold()
            ),
        }
    }

    fn display(&self) {
        let report = miette::Report::new(self.clone());
        eprintln!("{:?}", report);
    }
}

const LIVE_RELOAD_SCRIPT: &str = r#"<script>
(function() {
    let reloading = false;
    let wasConnected = false;
    function connect() {
        if (reloading) return;
        const ws = new WebSocket('ws://' + window.location.host + '/__hugs_live_reload');
        ws.onopen = function() {
            if (wasConnected && !reloading) {
                console.log('[hugs] reconnected to dev server, reloading...');
                reloading = true;
                window.location.reload();
            } else {
                console.log('[hugs] connected to dev server');
            }
            wasConnected = true;
        };
        ws.onmessage = function(event) {
            if (event.data === 'reload' && !reloading) {
                console.log('[hugs] file change detected, reloading...');
                reloading = true;
                window.location.reload();
            }
        };
        ws.onclose = function() {
            if (!reloading) {
                console.log('[hugs] disconnected from dev server, retrying in 1s...');
                setTimeout(connect, 1000);
            }
        };
        ws.onerror = function() {
            ws.close();
        };
    }
    connect();
})();
</script>"#;

pub struct DevAppState {
    pub app_data: RwLock<Option<AppData>>,
    /// Stores an error when site data couldn't be loaded (startup or reload error)
    /// When this is Some, all page requests will show this error
    pub startup_error: RwLock<Option<HugsError>>,
    pub reload_tx: broadcast::Sender<()>,
    pub minify_config: MinifyConfig,
}

struct LiveReloadWs {
    reload_rx: broadcast::Receiver<()>,
}

impl LiveReloadWs {
    fn new(mut reload_rx: broadcast::Receiver<()>) -> Self {
        // Drain any pending messages so we don't immediately reload on connect
        while reload_rx.try_recv().is_ok() {}
        Self { reload_rx }
    }
}

impl Actor for LiveReloadWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(100), |act, ctx| {
            match act.reload_rx.try_recv() {
                Ok(()) => {
                    ctx.text("reload");
                }
                // Ignore lagged/empty/closed - don't reload on stale messages
                Err(_) => {}
            }
        });
    }
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for LiveReloadWs {
    fn handle(&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Close(_)) => ctx.stop(),
            _ => {}
        }
    }
}

#[get("/__hugs_live_reload")]
async fn live_reload_ws(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<Arc<DevAppState>>,
) -> std::result::Result<HttpResponse, actix_web::Error> {
    let reload_rx = state.reload_tx.subscribe();
    ws::start(LiveReloadWs::new(reload_rx), &req, stream)
}

#[get("/theme.css")]
async fn theme(state: web::Data<Arc<DevAppState>>) -> HttpResponse {
    // Check for startup error
    if let Some(error) = state.startup_error.read().await.as_ref() {
        return HttpResponse::InternalServerError()
            .content_type(ContentType::html())
            .body(render_error_html(error, LIVE_RELOAD_SCRIPT));
    }

    let app_data_guard = state.app_data.read().await;
    let app_data = match app_data_guard.as_ref() {
        Some(data) => data,
        None => return HttpResponse::InternalServerError().body("I couldn't load the site data"),
    };
    let css = minify_css_content(&app_data.theme_css, &state.minify_config);
    HttpResponse::Ok()
        .content_type(ContentType(mime_guess::mime::TEXT_CSS_UTF_8))
        .body(css)
}

/// Handle cache-busted theme CSS (e.g., /theme.a1b2c3f4.css)
/// In dev mode, we serve the theme CSS regardless of the hash value
#[get("/theme.{hash}.css")]
async fn theme_hashed(state: web::Data<Arc<DevAppState>>) -> HttpResponse {
    // Check for startup error
    if let Some(error) = state.startup_error.read().await.as_ref() {
        return HttpResponse::InternalServerError()
            .content_type(ContentType::html())
            .body(render_error_html(error, LIVE_RELOAD_SCRIPT));
    }

    let app_data_guard = state.app_data.read().await;
    let app_data = match app_data_guard.as_ref() {
        Some(data) => data,
        None => return HttpResponse::InternalServerError().body("I couldn't load the site data"),
    };
    let css = minify_css_content(&app_data.theme_css, &state.minify_config);
    HttpResponse::Ok()
        .content_type(ContentType(mime_guess::mime::TEXT_CSS_UTF_8))
        .body(css)
}

#[get("/sitemap.xml")]
async fn sitemap(state: web::Data<Arc<DevAppState>>) -> HttpResponse {
    // Check for startup error
    if let Some(error) = state.startup_error.read().await.as_ref() {
        return HttpResponse::InternalServerError()
            .content_type(ContentType::html())
            .body(render_error_html(error, LIVE_RELOAD_SCRIPT));
    }

    let app_data_guard = state.app_data.read().await;
    let app_data = match app_data_guard.as_ref() {
        Some(data) => data,
        None => return HttpResponse::InternalServerError().body("I couldn't load the site data"),
    };
    match generate_sitemap(&app_data.pages, &app_data.config.site) {
        Ok(xml) => HttpResponse::Ok()
            .content_type(ContentType::xml())
            .body(xml),
        Err(e) => HttpResponse::InternalServerError()
            .content_type(ContentType::html())
            .body(render_error_html(&e, LIVE_RELOAD_SCRIPT)),
    }
}

/// Try to match a URL path against dynamic page patterns
/// Returns (source_file_path, DynamicContext) if a match is found
fn match_dynamic_page(url_path: &str, app_data: &AppData) -> Option<(String, DynamicContext)> {
    use serde_yaml::Value as YamlValue;

    for def in app_data.dynamic_defs.iter() {
        // Convert source path to a pattern (e.g., "blog/[slug].md" -> regex to match "blog/*")
        let source_path_str = def.source_path.to_string_lossy();
        let source_without_ext = source_path_str.strip_suffix(".md").unwrap_or(&source_path_str);

        // Create the pattern by replacing [param] with a capture group
        let placeholder = format!("[{}]", def.param_name);

        // Check if the URL could match this pattern
        // Split both paths into segments and compare
        let pattern_segments: Vec<&str> = source_without_ext.split('/').collect();
        let url_segments: Vec<&str> = url_path.split('/').filter(|s| !s.is_empty()).collect();

        if pattern_segments.len() != url_segments.len() {
            continue;
        }

        let mut matched_value: Option<String> = None;
        let mut all_match = true;

        for (pattern_seg, url_seg) in pattern_segments.iter().zip(url_segments.iter()) {
            if *pattern_seg == placeholder {
                matched_value = Some(url_seg.to_string());
            } else if pattern_seg != url_seg {
                all_match = false;
                break;
            }
        }

        if all_match {
            if let Some(value_str) = matched_value {
                // Check if this value is in the allowed list
                let value_yaml = YamlValue::String(value_str.clone());
                let value_matches = def.param_values.iter().any(|v| {
                    match v {
                        YamlValue::String(s) => s == &value_str,
                        YamlValue::Number(n) => n.to_string() == value_str,
                        _ => false,
                    }
                });

                if value_matches {
                    // Find the actual YAML value (to preserve type)
                    let param_value = def.param_values.iter()
                        .find(|v| match v {
                            YamlValue::String(s) => s == &value_str,
                            YamlValue::Number(n) => n.to_string() == value_str,
                            _ => false,
                        })
                        .cloned()
                        .unwrap_or(value_yaml);

                    return Some((
                        def.source_path.to_string_lossy().to_string(),
                        DynamicContext {
                            param_name: def.param_name.clone(),
                            param_value,
                        },
                    ));
                }
            }
        }
    }

    None
}

#[get("/{tail:.*}")]
async fn page(path: web::Path<String>, state: web::Data<Arc<DevAppState>>) -> HttpResponse {
    // Check for startup error first - if there's an error, show it for all requests
    if let Some(error) = state.startup_error.read().await.as_ref() {
        return HttpResponse::InternalServerError()
            .content_type(ContentType::html())
            .body(render_error_html(error, LIVE_RELOAD_SCRIPT));
    }

    let app_data_guard = state.app_data.read().await;
    let app_data = match app_data_guard.as_ref() {
        Some(data) => data,
        None => {
            // No app data and no error - this shouldn't happen, but handle gracefully
            return HttpResponse::InternalServerError()
                .content_type(ContentType::html())
                .body("I couldn't load the site data");
        }
    };

    // Normalize path by trimming trailing slashes
    let path_str = path.trim_end_matches('/');

    if let Some(response) = try_serve_static_file(path_str, &app_data).await {
        return response;
    }

    // First try to resolve as a static page
    match resolve_path_to_doc(path_str, &app_data).await {
        Ok(Some((frontmatter, doc_html, resolvable_path, frontmatter_json))) => {
            match render_page_html(
                &frontmatter,
                &frontmatter_json,
                &doc_html,
                &resolvable_path,
                &app_data,
                LIVE_RELOAD_SCRIPT,
            ) {
                Ok(html_out) => {
                    let final_html = minify_html_content(&html_out, &state.minify_config);
                    HttpResponse::Ok()
                        .content_type(ContentType::html())
                        .body(final_html)
                }
                Err(e) => HttpResponse::InternalServerError()
                    .content_type(ContentType::html())
                    .body(render_error_html(&e, LIVE_RELOAD_SCRIPT)),
            }
        }
        Ok(None) => {
            // Static page not found - try to match against dynamic pages
            if let Some((source_path, dynamic_ctx)) = match_dynamic_page(path_str, &app_data) {
                match resolve_dynamic_doc(&source_path, &dynamic_ctx, &app_data).await {
                    Ok((frontmatter, doc_html, _resolvable_path, frontmatter_json)) => {
                        // Build the page URL from the request path
                        let page_url = format!("/{}", path_str);
                        match render_dynamic_page_html(
                            &frontmatter,
                            &frontmatter_json,
                            &doc_html,
                            &page_url,
                            &app_data,
                            LIVE_RELOAD_SCRIPT,
                        ) {
                            Ok(html_out) => {
                                let final_html = minify_html_content(&html_out, &state.minify_config);
                                return HttpResponse::Ok()
                                    .content_type(ContentType::html())
                                    .body(final_html);
                            }
                            Err(e) => {
                                return HttpResponse::InternalServerError()
                                    .content_type(ContentType::html())
                                    .body(render_error_html(&e, LIVE_RELOAD_SCRIPT));
                            }
                        }
                    }
                    Err(e) => {
                        return HttpResponse::InternalServerError()
                            .content_type(ContentType::html())
                            .body(render_error_html(&e, LIVE_RELOAD_SCRIPT));
                    }
                }
            }

            // No match found - show 404 page
            if let Some(html) = render_notfound_page(&app_data, LIVE_RELOAD_SCRIPT).await {
                let final_html = minify_html_content(&html, &state.minify_config);
                HttpResponse::NotFound()
                    .content_type(ContentType::html())
                    .body(final_html)
            } else {
                HttpResponse::NotFound()
                    .content_type(ContentType::html())
                    .body("Not Found")
            }
        }
        Err(e) => {
            // Error occurred while processing - show error in page
            HttpResponse::InternalServerError()
                .content_type(ContentType::html())
                .body(render_error_html(&e, LIVE_RELOAD_SCRIPT))
        }
    }
}

fn start_file_watcher(
    site_path: PathBuf,
    state: Arc<DevAppState>,
) -> notify::Result<RecommendedWatcher> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(100);

    let watcher = RecommendedWatcher::new(
        move |res: std::result::Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                // Only trigger on actual file modifications (write, content change)
                let dominated = matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Data(_))
                );
                if dominated {
                    let _ = tx.blocking_send(());
                }
            }
        },
        Config::default(),
    )?;

    let site_path_clone = site_path.clone();
    tokio::spawn(async move {
        const DEBOUNCE_MS: u64 = 150;

        loop {
            // Wait for the first event
            if rx.recv().await.is_none() {
                break;
            }

            // Debounce: wait for events to stop arriving
            loop {
                let sleep = std::pin::pin!(tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)));

                tokio::select! {
                    result = rx.recv() => {
                        if result.is_none() {
                            return;
                        }
                        // Event received - continue loop to reset timer
                    }
                    _ = sleep => {
                        break; // Quiet period elapsed
                    }
                }
            }

            console::status_cyan("Watching", "file change detected, reloading...");

            match AppData::load(site_path_clone.clone(), "dev").await {
                Ok(new_data) => {
                    // Clear any previous error
                    {
                        let mut error = state.startup_error.write().await;
                        *error = None;
                    }
                    // Update app data
                    {
                        let mut app_data = state.app_data.write().await;
                        *app_data = Some(new_data);
                    }
                    let _ = state.reload_tx.send(());
                    console::status("Reloaded", "site data");
                }
                Err(e) => {
                    console::warn("couldn't reload site data");
                    let report = miette::Report::new(e.clone());
                    eprintln!("{:?}", report);

                    // Store the error so it's shown in the browser
                    {
                        let mut error = state.startup_error.write().await;
                        *error = Some(e);
                    }
                    // Still trigger reload so the browser refreshes and shows the error
                    let _ = state.reload_tx.send(());
                }
            }
        }
    });

    Ok(watcher)
}

pub async fn run_dev_server(path: PathBuf, requested_port: Option<u16>) -> Result<()> {
    console::status("Starting", "development server with live reload");
    console::status("Watching", path.display());

    let (reload_tx, _) = broadcast::channel(16);

    // Try to load the site data, but don't fail if there's an error
    // Instead, store the error and show it in the browser
    let (app_data, startup_error, minify_config) = match AppData::load(path.clone(), "dev").await {
        Ok(data) => {
            let minify = MinifyConfig::new(data.config.build.minify);
            (Some(data), None, minify)
        }
        Err(e) => {
            // Print the error to terminal as well
            console::warn("couldn't load site data");
            let report = miette::Report::new(e.clone());
            eprintln!("{:?}", report);
            console::status_cyan("Waiting", "for file changes to retry...");

            // Use default minify config when we can't load the site
            (None, Some(e), MinifyConfig::new(false))
        }
    };

    let state = Arc::new(DevAppState {
        app_data: RwLock::new(app_data),
        startup_error: RwLock::new(startup_error),
        reload_tx,
        minify_config,
    });

    let mut watcher = start_file_watcher(path.clone(), Arc::clone(&state))
        .map_err(|e| HugsError::WatcherInit { cause: e })?;

    watcher
        .watch(&path, RecursiveMode::Recursive)
        .map_err(|e| HugsError::WatcherPath {
            path: (&path).into(),
            cause: e,
        })?;

    let (server, actual_port) = try_bind_server(Arc::clone(&state), &path, requested_port)?;

    console::status("Listening", format!("http://127.0.0.1:{}", actual_port));

    // Display warning if port changed (after the server starting log)
    if requested_port.is_none() && actual_port != DEFAULT_PORT {
        PortChangedWarning::new(actual_port).display();
    }

    server
        .await
        .map_err(|e| HugsError::ServerRuntime { cause: e })?;

    drop(watcher);
    Ok(())
}

/// Attempt to bind to a port, retrying with incrementing ports if port was not explicitly specified
fn try_bind_server(
    state: Arc<DevAppState>,
    path: &PathBuf,
    requested_port: Option<u16>,
) -> Result<(actix_web::dev::Server, u16)> {
    if let Some(port) = requested_port {
        // Port was explicitly specified: fail immediately if unavailable
        let state_for_server = Arc::clone(&state);
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(Arc::clone(&state_for_server)))
                .service(live_reload_ws)
                .service(theme)
                .service(theme_hashed)
                .service(sitemap)
                .service(page)
        })
        .bind(("127.0.0.1", port))
        .map_err(|e| HugsError::port_bind(path, port, e))?;

        Ok((server.run(), port))
    } else {
        let port: u16 = DEFAULT_PORT;

        // Default port: try subsequent ports until one is available
        for attempt in 0..MAX_PORT_RETRIES {
            let try_port = match port.checked_add(attempt) {
                Some(p) => p,
                None => break, // Port overflow, stop trying
            };

            let state_for_server = Arc::clone(&state);
            match HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(Arc::clone(&state_for_server)))
                    .service(live_reload_ws)
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
                    // Try next port
                    continue;
                }
            }
        }

        // All retries exhausted
        let end_port = port.saturating_add(MAX_PORT_RETRIES - 1);
        Err(HugsError::NoAvailablePort {
            start_port: port.into(),
            end_port: end_port.into(),
        })
    }
}
