use std::collections::HashMap;
use std::time::{Duration, Instant};

use axum::Json;
use axum::http::StatusCode;
use deadpool_redis::redis::AsyncCommands;
use diesel_async::RunQueryDsl;
use serde::Serialize;

use crate::AppState;

const POSTGRES_TIMEOUT: Duration = Duration::from_secs(5);
const REDIS_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Serialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub components: HashMap<String, ComponentHealth>,
}

pub async fn handler(state: AppState) -> (StatusCode, Json<HealthResponse>) {
    let mut components = HashMap::new();

    let (pg_rw, pg_ro, redis_main) = tokio::join!(
        check_postgres(&state.pool),
        check_postgres(&state.read_only_pool),
        check_redis(&state.cache_pool),
    );

    components.insert("postgres_rw".to_string(), pg_rw);
    components.insert("postgres_ro".to_string(), pg_ro);
    components.insert("redis_main".to_string(), redis_main);

    let is_shutting_down = state.shutdown.is_shutting_down();

    let critical_unhealthy = is_shutting_down
        || components
            .get("postgres_rw")
            .is_some_and(|c| c.status == HealthStatus::Unhealthy)
        || components
            .get("redis_main")
            .is_some_and(|c| c.status == HealthStatus::Unhealthy);

    let any_unhealthy = components
        .values()
        .any(|c| c.status == HealthStatus::Unhealthy);

    let overall = if critical_unhealthy {
        HealthStatus::Unhealthy
    } else if any_unhealthy {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };

    let status_code = if overall == HealthStatus::Unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (
        status_code,
        Json(HealthResponse {
            status: overall,
            components,
        }),
    )
}

async fn check_postgres(pool: &postgres::connection::Pool) -> ComponentHealth {
    let start = Instant::now();
    let result = tokio::time::timeout(POSTGRES_TIMEOUT, async {
        let mut conn = pool.get_owned().await.map_err(|e| e.to_string())?;
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    })
    .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(())) => ComponentHealth {
            status: HealthStatus::Healthy,
            latency_ms: Some(latency_ms),
            error: None,
        },
        Ok(Err(e)) => ComponentHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: Some(latency_ms),
            error: Some(e),
        },
        Err(_) => ComponentHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: Some(latency_ms),
            error: Some("timeout".to_string()),
        },
    }
}

async fn check_redis(pool: &redis::connection::Pool) -> ComponentHealth {
    let start = Instant::now();
    let result = tokio::time::timeout(REDIS_TIMEOUT, async {
        let mut conn = pool.get().await.map_err(|e| e.to_string())?;
        let _: () = conn.ping().await.map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    })
    .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(())) => ComponentHealth {
            status: HealthStatus::Healthy,
            latency_ms: Some(latency_ms),
            error: None,
        },
        Ok(Err(e)) => ComponentHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: Some(latency_ms),
            error: Some(e),
        },
        Err(_) => ComponentHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: Some(latency_ms),
            error: Some("timeout".to_string()),
        },
    }
}
