use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use axum::{routing::get, Router};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
mod middleware;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use axum::body::Body;
use http::{HeaderValue, StatusCode};
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::{GovernorError, GovernorLayer};

mod config;
mod db;
mod error;
mod i18n;
mod routes;
mod services;

use config::Config;
use services::{discord::DiscordService, init, telegram::TelegramService, twitch::TwitchService};

pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: Config,
    pub twitch: TwitchService,
    pub telegram: Arc<RwLock<Option<TelegramService>>>,
    pub discord: Arc<RwLock<Option<DiscordService>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "twitch_notifications=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;

    tracing::info!("Starting Twitch Notifications Service");

    // Initialize database
    let pool = init::init_db(&config).await?;

    // Initialize services
    let twitch = TwitchService::new(&config).await?;

    let app_state = Arc::new(AppState {
        db: pool,
        config: config.clone(),
        twitch,
        telegram: Arc::new(RwLock::new(None)),
        discord: Arc::new(RwLock::new(None)),
    });

    // Initialize optional integrations (Telegram, Discord)
    init::initialize_optional_integrations(&app_state).await;

    // Create shutdown notifier for background workers and std threads
    let (shutdown_tx, _shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
    let thread_shutdown = Arc::new(AtomicBool::new(false));

    // Spawn background workers (returns JoinHandles so we can await shutdown)
    let bg_handles = init::spawn_background_workers(app_state.clone(), shutdown_tx.clone());

    // Build rate limiters for public endpoints (auth, webhooks)
    // Build auth rate limiter with a custom error handler.
    // The error handler returns a proper 429 status and Retry-After header when limits are exceeded.
    let mut auth_builder = GovernorConfigBuilder::default();
    auth_builder.per_second(config.rate_limit.auth_per_second.into());
    auth_builder.burst_size(config.rate_limit.auth_burst);
    auth_builder.key_extractor(SmartIpKeyExtractor);
    auth_builder.error_handler(|error: GovernorError| -> http::Response<Body> {
        match error {
            GovernorError::TooManyRequests { wait_time, headers } => {
                // `wait_time` is provided as seconds
                let retry_after = wait_time;

                // Use the same error shape as `AppError::RateLimited -> IntoResponse`
                let body = serde_json::json!({
                    "error": {
                        "code": "RATE_LIMITED",
                        "message": "Rate limit exceeded",
                        "details": { "retry_after_seconds": retry_after }
                    }
                })
                .to_string();

                let mut resp = http::Response::new(Body::from(body));
                *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;

                // Ensure clients see JSON
                resp.headers_mut().insert(
                    http::header::CONTENT_TYPE,
                    http::HeaderValue::from_static("application/json"),
                );

                // Include any headers provided by the governor (e.g., X-RateLimit-* if enabled)
                if let Some(hmap) = headers {
                    for (name, value) in hmap.iter() {
                        resp.headers_mut().append(name.clone(), value.clone());
                    }
                }

                // Retry-After (seconds)
                resp.headers_mut().insert(
                    http::header::RETRY_AFTER,
                    http::HeaderValue::from_str(&retry_after.to_string()).unwrap(),
                );

                resp
            }
            GovernorError::UnableToExtractKey => {
                let body = serde_json::json!({
                    "error": {
                        "code": "INVALID_REQUEST",
                        "message": "Unable to determine client IP for rate limiting"
                    }
                })
                .to_string();

                let mut resp = http::Response::new(Body::from(body));
                *resp.status_mut() = StatusCode::BAD_REQUEST;
                resp.headers_mut().insert(
                    http::header::CONTENT_TYPE,
                    http::HeaderValue::from_static("application/json"),
                );
                resp
            }
            GovernorError::Other { code, msg, headers } => {
                let body = msg.unwrap_or_else(|| "Rate limiting error".to_string());
                let mut resp = http::Response::new(Body::from(body));
                let status = StatusCode::from_u16(code.as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                *resp.status_mut() = status;
                if let Some(hmap) = headers {
                    for (name, value) in hmap.iter() {
                        resp.headers_mut().append(name.clone(), value.clone());
                    }
                }
                resp
            }
        }
    });

    let auth_gov_conf = Arc::new(
        auth_builder
            .finish()
            .ok_or_else(|| anyhow::anyhow!("Failed to build auth governor config"))?,
    );

    // Background cleanup for auth limiter storage
    let auth_cleaner = {
        let limiter = auth_gov_conf.limiter().clone();
        let interval = Duration::from_secs(60);
        let flag = thread_shutdown.clone();
        std::thread::spawn(move || {
            // Use smaller sleep granularity to allow quick shutdown.
            let tick = Duration::from_secs(1);
            loop {
                for _ in 0..interval.as_secs() {
                    if flag.load(Ordering::SeqCst) {
                        tracing::info!("Auth rate limiter cleanup thread exiting");
                        return;
                    }
                    std::thread::sleep(tick);
                }
                tracing::debug!("auth rate limiter size: {}", limiter.len());
                limiter.retain_recent();
            }
        })
    };

    // Apply the auth rate limiter layer
    let auth_rate_layer = GovernorLayer {
        config: auth_gov_conf.clone(),
    };

    // Webhooks limiter
    let mut webhooks_builder = GovernorConfigBuilder::default();
    webhooks_builder.per_second(config.rate_limit.webhook_per_second.into());
    webhooks_builder.burst_size(config.rate_limit.webhook_burst);
    webhooks_builder.key_extractor(SmartIpKeyExtractor);
    webhooks_builder.error_handler(|error: GovernorError| -> http::Response<Body> {
        match error {
            GovernorError::TooManyRequests { wait_time, headers } => {
                // `wait_time` is provided as seconds
                let retry_after = wait_time;
                let body = serde_json::json!({
                    "error": "rate_limit_exceeded",
                    "retry_after_seconds": retry_after
                })
                .to_string();

                let mut resp = http::Response::new(Body::from(body));
                *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;

                if let Some(hmap) = headers {
                    for (name, value) in hmap.iter() {
                        resp.headers_mut().append(name.clone(), value.clone());
                    }
                }

                resp.headers_mut().insert(
                    http::header::RETRY_AFTER,
                    http::HeaderValue::from_str(&retry_after.to_string()).unwrap(),
                );

                resp
            }
            GovernorError::UnableToExtractKey => {
                let mut resp = http::Response::new(Body::from(
                    "Unable to determine client IP for rate limiting",
                ));
                *resp.status_mut() = StatusCode::BAD_REQUEST;
                resp
            }
            GovernorError::Other { code, msg, headers } => {
                let body = msg.unwrap_or_else(|| "Rate limiting error".to_string());
                let mut resp = http::Response::new(Body::from(body));
                let status = StatusCode::from_u16(code.as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                *resp.status_mut() = status;
                if let Some(hmap) = headers {
                    for (name, value) in hmap.iter() {
                        resp.headers_mut().append(name.clone(), value.clone());
                    }
                }
                resp
            }
        }
    });
    let webhooks_gov_conf = Arc::new(
        webhooks_builder
            .finish()
            .ok_or_else(|| anyhow::anyhow!("Failed to build webhooks governor config"))?,
    );

    // Background cleanup for webhooks limiter storage
    let webhooks_cleaner = {
        let limiter = webhooks_gov_conf.limiter().clone();
        let interval = Duration::from_secs(60);
        let flag = thread_shutdown.clone();
        std::thread::spawn(move || {
            let tick = Duration::from_secs(1);
            loop {
                for _ in 0..interval.as_secs() {
                    if flag.load(Ordering::SeqCst) {
                        tracing::info!("Webhooks rate limiter cleanup thread exiting");
                        return;
                    }
                    std::thread::sleep(tick);
                }
                tracing::debug!("webhooks rate limiter size: {}", limiter.len());
                limiter.retain_recent();
            }
        })
    };

    let webhooks_rate_layer = GovernorLayer {
        config: webhooks_gov_conf.clone(),
    };

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(routes::health::health_check))
        // Auth routes (apply rate limiting for public auth endpoints)
        .nest("/api/auth", routes::auth::router().layer(auth_rate_layer))
        // User routes (search, etc.)
        .nest("/api/users", routes::users::router())
        // User settings routes
        .nest("/api/settings", routes::settings::router())
        // Notification routes
        .nest("/api/notifications", routes::notifications::router())
        // Integration routes (Telegram, Discord)
        .nest("/api/integrations", routes::integrations::router())
        // Calendar sync endpoints (manual trigger / status)
        .nest("/api/calendar", routes::calendar::router())
        // Twitch EventSub webhooks (apply rate limiting)
        .nest(
            "/webhooks",
            routes::webhooks::router().layer(webhooks_rate_layer),
        )
        // Add shared state
        .with_state(app_state.clone())
        // CSP middleware: set Content-Security-Policy headers
        .layer(axum::middleware::from_fn(middleware::csp::csp_middleware))
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(
                    config
                        .server
                        .frontend_url
                        .parse::<HeaderValue>()
                        .expect("Invalid FRONTEND_URL for CORS"),
                )
                .allow_methods([
                    http::Method::GET,
                    http::Method::POST,
                    http::Method::PUT,
                    http::Method::DELETE,
                    http::Method::OPTIONS,
                    http::Method::PATCH,
                ])
                .allow_headers([
                    http::header::CONTENT_TYPE,
                    http::header::AUTHORIZATION,
                    http::header::ACCEPT,
                ])
                .allow_credentials(true),
        );

    // Start server
    let host = config.server.host.clone();
    let port = config.server.port;
    let addr = format!("{}:{}", host, port);

    tracing::info!("Server listening on {}", addr);

    // Start server using axum `serve` helper. We also spawn a signal listener
    // and select between the server future and the signal future. When a
    // shutdown signal is received we notify background workers and threads
    // and then drop the server future (which stops accepting new connections).
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let server_fut = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );

    let shutdown_tx_clone = shutdown_tx.clone();
    let thread_shutdown_clone = thread_shutdown.clone();

    let signal_fut = async move {
        let ctrl_c = tokio::signal::ctrl_c();

        #[cfg(unix)]
        {
            let mut term =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to bind SIGTERM");
            tokio::select! {
                _ = ctrl_c => {},
                _ = term.recv() => {},
            }
        }

        #[cfg(not(unix))]
        {
            ctrl_c.await.expect("Failed to bind Ctrl+C");
        }

        tracing::info!("Shutdown signal received, notifying background workers and threads");
        let _ = shutdown_tx_clone.send(());
        thread_shutdown_clone.store(true, Ordering::SeqCst);
    };

    tokio::select! {
        res = server_fut => {
            if let Err(e) = res {
                tracing::error!("Server error: {}", e);
            }
        }
        _ = signal_fut => {
            tracing::info!("Signal handler completed; server future dropped to stop accepting new connections");
        }
    }

    // Give background workers some time to finish their work.
    let shutdown_wait = Duration::from_secs(15);
    tracing::info!(
        "Waiting up to {}s for background workers to exit",
        shutdown_wait.as_secs()
    );

    // Wait for tokio background workers to finish with a timeout.
    let bg_wait = async {
        for h in bg_handles {
            let _ = h.await;
        }
    };
    let _ = tokio::time::timeout(shutdown_wait, bg_wait).await;

    // Join std threads; they check `thread_shutdown` and should exit quickly.
    if let Err(e) = auth_cleaner.join() {
        tracing::warn!("Auth cleanup thread join failed: {:?}", e);
    }
    if let Err(e) = webhooks_cleaner.join() {
        tracing::warn!("Webhooks cleanup thread join failed: {:?}", e);
    }

    tracing::info!("Shutdown complete");
    Ok(())
}
