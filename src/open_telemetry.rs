use once_cell::sync::Lazy;
use opentelemetry::{ KeyValue};
use opentelemetry::logs::LogError;
use opentelemetry::metrics::MetricsError;
use opentelemetry::propagation::{Extractor, Injector};
use opentelemetry::trace::TraceError;
use opentelemetry_otlp::{ WithExportConfig};
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::{Resource, runtime};
use opentelemetry_sdk::trace::{Config, TracerProvider};


pub(crate) fn init_logs(otlp_endpoint: &str, resource: Resource) -> Result<LoggerProvider, LogError> {

    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_resource(resource)

        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .install_batch(runtime::Tokio)
}

pub(crate) fn init_tracer_provider(otlp_endpoint: &str,resource: Resource) -> Result<TracerProvider, TraceError> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()

                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .with_trace_config(Config::default().with_resource(resource))
        .install_batch(runtime::Tokio)
}
pub(crate) fn init_metrics(otlp_endpoint: &str, resource: Resource) -> Result<opentelemetry_sdk::metrics::SdkMeterProvider, MetricsError> {

    opentelemetry_otlp::new_pipeline()
        .metrics(runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
                // .with_export_config(export_config),
        )
        .with_resource(resource)
        .build()
}