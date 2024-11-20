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

    open_telemetry::init_otel(&app_config).await;

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
    // logger_provider.shutdown();
    Ok(())
}
