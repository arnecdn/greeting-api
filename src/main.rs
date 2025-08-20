use crate::settings::Settings;
use actix_web::{web, App, HttpServer};
use futures_util::join;
use greeting_db_api::greeting_query::GreetingQueryRepositoryImpl;
use log::error;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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

    greeting_otel::init_otel(&app_config.otel_collector.oltp_endpoint,"greeting_api", &app_config.kube.my_pod_name).await;

    let pool = Box::new(
        greeting_db_api::init_db(app_config.db.database_url.clone())
            .await
            .expect("Expected db pool"),
    );

    greeting_db_api::migrate(&pool.clone())
        .await
        .expect("Failed to migrate db");

    let log_generator_handle = greeting::generate_logg(pool.clone());

    let query_repo = Box::new(
        GreetingQueryRepositoryImpl::new(pool)
            .await
            .expect("Failed creating pool"),
    );
    let querier_data = web::Data::new(query_repo);

    let server_handle = HttpServer::new(move || {
        App::new()
            .app_data(querier_data.clone())
            .service(greeting::list_log_entries)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run();

    let (log_result, server_result) = join!(log_generator_handle, server_handle);

    if let Err(e) = log_result {
        error!("Log generator failed: {:?}", e);
    }
    if let Err(e) = server_result {
        error!("Server failed: {:?}", e);
    }
    // global::shutdown_tracer_provider();
    // logger_provider.shutdown();
    Ok(())
}
