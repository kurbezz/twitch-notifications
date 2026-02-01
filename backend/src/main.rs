use std::net::SocketAddr;
use std::sync::Arc;
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

    // Spawn background workers
    init::spawn_background_workers(app_state.clone());

    // Build rate limiters for public endpoints (auth, webhooks)
    // Build auth rate limiter with a custom error handler.
    // The error handler returns a proper 429 status and Retry-After header when limits are exceeded.
    let mut auth_builder = GovernorConfigBuilder::default();
    auth_builder.per_second(config.rate_limit.auth_per_second.into());
    auth_builder.burst_size(config.rate_limit.auth_burst.into());
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
    {
        let limiter = auth_gov_conf.limiter().clone();
        let interval = Duration::from_secs(60);
        std::thread::spawn(move || loop {
            std::thread::sleep(interval);
            tracing::debug!("auth rate limiter size: {}", limiter.len());
            limiter.retain_recent();
        });
    }

    // Apply the auth rate limiter layer
    let auth_rate_layer = GovernorLayer {
        config: auth_gov_conf.clone(),
    };

    // Webhooks limiter
    let mut webhooks_builder = GovernorConfigBuilder::default();
    webhooks_builder.per_second(config.rate_limit.webhook_per_second.into());
    webhooks_builder.burst_size(config.rate_limit.webhook_burst.into());
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
    {
        let limiter = webhooks_gov_conf.limiter().clone();
        let interval = Duration::from_secs(60);
        std::thread::spawn(move || loop {
            std::thread::sleep(interval);
            tracing::debug!("webhooks rate limiter size: {}", limiter.len());
            limiter.retain_recent();
        });
    }

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

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
