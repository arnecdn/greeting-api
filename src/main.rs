use actix_web::{web, App, HttpServer};
use futures_util::join;
use opentelemetry::{global, KeyValue};
use opentelemetry::trace::TracerProvider;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::settings::Settings;

mod settings;
mod db;
mod greeting;
mod open_telemetry;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        info(description = "Greeting Api description"),
        paths(greeting::list_log_entries, greeting::list_log_entries),
        // components(schemas(api::GreetingDto))
    )]

    struct ApiDoc;

    let app_config = Settings::new();

    let (logger_provider,_) = init_otel(&app_config).await;

    let pool = Box::new(db::init_db(app_config.db.database_url.clone()).await.expect("Expected db pool"));

    let log_generator_handle = greeting::generate_logg(pool.clone());

    let server_handle = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(greeting::list_log_entries)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )

    })
        .bind(("127.0.0.1", 8080))?
        .run();

    join!(log_generator_handle, server_handle);

    global::shutdown_tracer_provider();
    logger_provider.shutdown();
    Ok(())
}

async fn init_otel(app_config: &Settings) -> (LoggerProvider, opentelemetry_sdk::trace::TracerProvider) {
    const APP_NAME: &'static str = "greeting_api";
    let resource = Resource::new(vec![KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        APP_NAME,
    ), KeyValue::new(
        opentelemetry_semantic_conventions::resource::K8S_POD_NAME,
        app_config.kube.my_pod_name.clone(),
    )]);

    let result = open_telemetry::init_tracer_provider(&app_config.otel_collector.oltp_endpoint, resource.clone());
    let tracer_provider = result.unwrap();
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Create a tracing layer with the configured tracer
    let tracer_layer = tracing_opentelemetry::layer().
        with_tracer(tracer_provider.tracer(APP_NAME));

    // Initialize logs and save the logger_provider.
    let logger_provider = open_telemetry::init_logs(&app_config.otel_collector.oltp_endpoint, resource.clone()).unwrap();
    // Create a new OpenTelemetryTracingBridge using the above LoggerProvider.
    let logger_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    let filter = EnvFilter::new("info")
        .add_directive("hyper=info".parse().unwrap())
        .add_directive("h2=info".parse().unwrap())
        .add_directive("tonic=info".parse().unwrap())
        .add_directive("reqwest=info".parse().unwrap());

    tracing_subscriber::registry()
        .with(logger_layer)
        .with(filter)
        .with(tracer_layer)
        .init();
    // let meter_provider = init_metrics(&app_config.otel_collector.oltp_endpoint).expect("Failed initializing metrics");
    // global::set_meter_provider(meter_provider);

    (logger_provider, tracer_provider)
}