use async_trait::async_trait;
use prometheus::{IntCounterVec, Registry, register_int_counter_vec};
use telemetry::metrics::TelemetryMetrics;

#[derive(Clone, Debug)]
pub struct ServerMetrics {
    pub registry: Registry,

    pub request_errors: IntCounterVec,
}

impl Default for ServerMetrics {
    fn default() -> Self {
        ServerMetrics::new(None)
            .expect("Failed to create default ServerMetrics")
    }
}

#[async_trait]
impl TelemetryMetrics for ServerMetrics {
    fn registry(&self) -> &Registry {
        &self.registry
    }

    fn metrics(&self) -> Option<Self> {
        Some(self.clone())
    }
}

impl ServerMetrics {
    pub fn new_with_random_prefix() -> anyhow::Result<Self> {
        ServerMetrics::new(Some(ServerMetrics::generate_random_prefix()))
    }

    pub fn new(prefix: Option<String>) -> anyhow::Result<Self> {
        let metric_prefix = prefix
            .clone()
            .map(|p| format!("{}_", p))
            .unwrap_or_default();

        let request_errors = register_int_counter_vec!(
            format!("{}request_errors", metric_prefix),
            "A metric counting request errors by handler and error code",
            &["handler", "error_code"],
        )
        .expect("metric must be created");

        let registry =
            Registry::new_custom(prefix, None).expect("registry to be created");
        registry.register(Box::new(request_errors.clone()))?;

        Ok(Self {
            registry,
            request_errors,
        })
    }

    pub fn record_error(&self, handler: &str, error_code: &str) {
        self.request_errors
            .with_label_values(&[handler, error_code])
            .inc();
    }
}
