use anyhow::Context;
use axum::{http::StatusCode, response::Json};
use icon_api::metrics::ServerMetrics;
use icon_api::shutdown::{ShutdownCoordinator, listen_for_shutdown_signals};
use serde_json::json;
use std::sync::Arc;
use telemetry::metrics::Telemetry;
use tower_http::{
    catch_panic::CatchPanicLayer, compression::CompressionLayer,
    trace::TraceLayer,
};

use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::*;

const VERSION: Option<&'static str> = option_env!("VERSION");
const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::embed_migrations!("./../../../db/migrations");

async fn fallback_handler() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "Not Found",
            "message": "The requested endpoint does not exist",
            "status": 404
        })),
    )
}

fn main() {
    let version = VERSION.unwrap_or("unknown").to_string();
    let config = icon_api::Config::load().expect("Failed to load config");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(async {
            if let Err(e) = setup(config, version).await {
                tracing::error!("Fatal error during setup: {e:#}");
                std::process::exit(1);
            }
        });
}

async fn setup(
    config: icon_api::Config,
    _version: String,
) -> anyhow::Result<()> {
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize tracing filter")?;

    let use_json = config.log_format != "pretty";

    if use_json {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_timer(UtcTime::rfc_3339())
            .with_target(true)
            .with_level(true)
            .json();
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    } else {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_level(true)
            .with_ansi(true)
            .pretty();
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    };

    let addr: String = format!("0.0.0.0:{}", config.api_service_port);
    tracing::info!("Starting icon-api service at: {addr}");

    let db_rw_url = config.database_url.clone();
    let db_ro_url = config.database_ro_url();

    // If Postgres or Redis is down at boot we log it and start anyway on a
    // lazily-connecting pool, rather than refusing to boot. The in-memory endpoints
    // keep working; DB endpoints return 503 until the dependency comes back.
    let db_pool = match postgres::connection::establish_connection(
        db_rw_url.clone(),
    )
    .await
    {
        Ok(pool) => {
            tracing::info!("Connected to Postgres (read-write)");
            // Migrations (including the icon seed) only run when the DB is
            // reachable; otherwise they are skipped until a healthy restart.
            let conn = pool
                .get_owned()
                .await
                .context("Failed to get connection from pool for migrations")?;
            postgres::connection::run_migrations(conn, MIGRATIONS)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
                .context("Failed to run database migrations")?;
            pool
        }
        Err(e) => {
            tracing::warn!(
                "Postgres (read-write) unavailable at startup; continuing in \
                 degraded mode, migrations skipped: {e:#}"
            );
            postgres::connection::build_lazy_pool(db_rw_url)
        }
    };

    let read_only_pool =
        match postgres::connection::establish_connection(db_ro_url.clone())
            .await
        {
            Ok(pool) => {
                tracing::info!("Connected to Postgres (read-only)");
                pool
            }
            Err(e) => {
                tracing::warn!(
                    "Postgres (read-only) unavailable at startup; continuing \
                     in degraded mode: {e:#}"
                );
                postgres::connection::build_lazy_pool(db_ro_url)
            }
        };

    let redis_pool = match redis::connection::establish_connection(
        config.redis_url.clone(),
    )
    .await
    {
        Ok(pool) => {
            tracing::info!("Connected to Redis");
            pool
        }
        Err(e) => {
            tracing::warn!(
                "Redis unavailable at startup; continuing in degraded mode, \
                 lookups will not be cached: {e:#}"
            );
            redis::connection::build_lazy_pool(config.redis_url.clone())
                .context("Failed to build Redis pool")?
        }
    };

    let shutdown = Arc::new(ShutdownCoordinator::new(
        db_pool.clone(),
        redis_pool.clone(),
    ));

    let metrics =
        ServerMetrics::new(None).context("Failed to create server metrics")?;
    let telemetry = Telemetry::new(Some(metrics))
        .await
        .context("Failed to create telemetry")?;
    telemetry
        .start()
        .await
        .context("Failed to start telemetry")?;
    tracing::info!("Initialized telemetry");

    let app_state = icon_api::AppState {
        telemetry,
        pool: db_pool,
        read_only_pool,
        cache_pool: redis_pool,
        config: Arc::new(config),
        shutdown: shutdown.clone(),
    };
    let app = axum::Router::new()
        .without_v07_checks()
        .route("/health", {
            let state = app_state.clone();
            axum::routing::get(move || {
                let state = state.clone();
                async move { icon_api::health::handler(state).await }
            })
        })
        .route(
            "/version",
            axum::routing::get(|| async { VERSION.unwrap_or("unknown") }),
        )
        .route("/metrics", {
            let telemetry = app_state.telemetry.clone();
            axum::routing::get(move || {
                let telemetry = telemetry.clone();
                async move {
                    (
                        axum::http::StatusCode::OK,
                        [(
                            axum::http::header::CONTENT_TYPE,
                            "text/plain; charset=utf-8",
                        )],
                        telemetry.get_metrics().await,
                    )
                }
            })
        })
        .nest(
            "/api/icons/v1",
            icon_api::get_icon_api_v1_routes(app_state.clone()),
        )
        .fallback(fallback_handler)
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CatchPanicLayer::new())
        .merge(icon_api::get_openapi_routes());

    let shutdown_handle = shutdown.clone();
    tokio::spawn(async move {
        listen_for_shutdown_signals().await;
        shutdown_handle.shutdown().await;
    });

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {addr}"))?;
    let shutdown_for_serve = shutdown.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_for_serve.wait_for_shutdown().await
        })
        .await
        .context("Server exited with error")?;

    Ok(())
}
