use std::sync::Arc;

use telemetry::metrics::Telemetry;
use uuid::Uuid;

use crate::icon_api::icon_api_error_v1::IconApiV1Error;
use crate::metrics::ServerMetrics;

/// Trait for handler error types that can be converted to [`IconApiV1Error`].
pub trait IntoIconApiV1Error {
    fn into_icon_api_v1_error(self, request_id: &Uuid) -> IconApiV1Error;
}

/// Records error metrics and converts handler errors to [`IconApiV1Error`].
///
/// Replaces the per-handler `record_err` closures with a single reusable type.
pub struct ErrorRecorder<'a> {
    telemetry: &'a Arc<Telemetry<ServerMetrics>>,
    handler_name: &'a str,
    request_id: &'a Uuid,
}

impl<'a> ErrorRecorder<'a> {
    pub fn new(
        telemetry: &'a Arc<Telemetry<ServerMetrics>>,
        handler_name: &'a str,
        request_id: &'a Uuid,
    ) -> Self {
        Self {
            telemetry,
            handler_name,
            request_id,
        }
    }

    pub fn record<E: IntoIconApiV1Error>(
        &self,
        code: &str,
        e: E,
    ) -> IconApiV1Error {
        self.telemetry.maybe_use_metrics(|m| {
            m.record_error(self.handler_name, code);
        });
        e.into_icon_api_v1_error(self.request_id)
    }
}
