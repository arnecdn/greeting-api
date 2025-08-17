use crate::settings::Settings;
use actix_web::{web, App, HttpServer};
use futures_util::join;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod db;
mod greeting;
mod settings;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        info(description = "Greeting Api description"),
        paths(greeting::list_log_entries),
        components(schemas(greeting::LoggQuery, greeting::LoggEntry))
    )]

    struct ApiDoc;

    let app_config = Settings::new();

    // greeting_otel::init_otel(&app_config.otel_collector.oltp_endpoint,"greeting_api", &app_config.kube.my_pod_name).await;
    let pool = web::Data::new(Box::new(
        db::init_db(app_config.db.database_url.clone())
            .await
            .expect("Expected db pool"),
    ));

    let log_generator_handle = greeting::generate_logg(pool.clone());

    let server_handle = HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .service(greeting::list_log_entries)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(("127.0.0.1", 8081))?
    .run();

    let r = join!(log_generator_handle, server_handle);

    // global::shutdown_tracer_provider();
    // logger_provider.shutdown();
    Ok(())
}
