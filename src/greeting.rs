use std::collections::HashMap;
use crate::greeting::ApiError::{ApplicationError, NotFound};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{get, web, HttpResponse, ResponseError};
use chrono::{DateTime, Utc};
use derive_more::Display;
use greeting_db_api::greeting_pg_trace::PgTraceContext;
use greeting_db_api::greeting_query::{
    GreetingQueryRepository, GreetingQueryRepositoryImpl, LoggQueryEntity,
};
use greeting_db_api::DbError;
use log::{error, info};
use once_cell::sync::Lazy;
use opentelemetry::trace::TraceContextExt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::fmt::{Display, Formatter};
use std::time::Duration;
use time::sleep;
use tokio::time;
use tracing::{instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use utoipa::ToSchema;
use uuid::Uuid;
use validator_derive::Validate;

#[derive(Validate, Serialize, Deserialize, Clone, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoggQuery {
    #[validate(range(min = 1))]
    offset: i64,
    #[validate(range(min = 1, max = 1000))]
    limit: i64,
    #[validate(regex(path = *DIRECTION, message = "Invalid direction"))]
    direction: String,
}

impl Display for LoggQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoggQuery {{ offset: {}, limit: {}, direction: {} }}",
            self.offset, self.limit, self.direction
        )
    }
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoggEntry {
    id: i64,
    greeting_id: i64,
    message_id: String,
    #[schema(value_type = String, format = DateTime)]
    created: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Greeting{
    id: i64,
    message: GreetingMessage,
    #[schema(value_type = String, format = DateTime)]
    created: DateTime<Utc>,

}
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct GreetingMessage {
    external_reference: String,
    message_id: String,
    to: String,
    from: String,
    heading: String,
    message: String,
    #[schema(value_type = String, format = DateTime)]
    created: String,
    #[schema(value_type = HashMap<String, String>)]
    events_created: HashMap<String, String>,
}
//{"to": "arne",
// "from": "arne",
// "created": "2026-01-06T10:20:56.210182",
// "heading": "chrismas carg",
// "message": "Happy christmas",
// "message_id": "019b92bb-0088-77f1-8b09-5d56dfa72bc4",
// "events_created":
// {"received_greeting": "2026-01-06T09:54:47.560879858"},
// "external_reference": "019b92d2-f012-7222-a099-0b73d5788b30"}
#[utoipa::path(
    get,
    path = "/greeting/{greeting_id}",
    responses(
        (status = 200, description = "Greetings", body = Greeting),
        (status = NOT_FOUND, description = "Greetings was not found")
    )
)]
#[get("/greeting/{greeting_id}")]
#[instrument(name = "greeting_message")]
pub async fn greeting_message(
    data: Data<Box<GreetingQueryRepositoryImpl>>,
    greeting_id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let pg_trace = generate_pg_trace_context();

    info!("Access greeting with id: {}", &greeting_id);
    let result = data.find_greeting(pg_trace, *greeting_id).await?;

    match result {
        Some(v) => Ok(HttpResponse::Ok().json(Greeting {
            id: v.id,
            message: serde_json::from_value::<GreetingMessage>(v.message).unwrap(),
            created: v.created,
        })),
        None => Err(NotFound("Not found".to_string()))
    }
}

static DIRECTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(asc|desc)$").unwrap());

#[utoipa::path(
    get,
    path = "/log",
    params(
        ("logg_query" = LoggQuery, Query,)
    ),
    responses(
        (status = 200, description = "Greetings", body = LoggEntry),
        (status = 204, description = "Greetings was not found")
    )
)]
#[get("/log")]
#[instrument(name = "log")]
pub async fn list_log_entries(
    data: Data<Box<GreetingQueryRepositoryImpl>>,
    logg_query: web::Query<LoggQuery>,
) -> Result<HttpResponse, ApiError> {
    let pg_trace = generate_pg_trace_context();

    info!("Access logg {}", &logg_query);

    let query = LoggQueryEntity {
        offset: logg_query.offset,
        limit: logg_query.limit,
        direction: logg_query.direction.clone(),
    };

    let result = data.list_log_entries(pg_trace, query).await?;

    let logg_list = result
        .iter()
        .map(|e| LoggEntry {
            id: e.id,
            greeting_id: e.greeting_id,
            message_id: Uuid::to_string( &e.message_id),
            created: e.created,
        })
        .collect::<Vec<_>>();

    match logg_list.is_empty() {
        true => {Ok((HttpResponse::NoContent()).json(logg_list))}
        false => {Ok(HttpResponse::Ok().json(logg_list))}
    }

}

#[utoipa::path(
    get,
    path = "/log/last",
    responses(
        (status = 200, description = "Greetings", body = LoggEntry),
        (status = 204, description = "No greetings was not found in log")
    )
)]
#[get("/log/last")]
#[instrument(name = "last_log_entry")]
pub async fn last_log_entry(
    data: Data<Box<GreetingQueryRepositoryImpl>>,
) -> Result<HttpResponse, ApiError> {
    let pg_trace = generate_pg_trace_context();

    let result = data.last_log_entry(pg_trace).await?;

    match result {
        Some(v) => Ok(HttpResponse::Ok().json(LoggEntry {
            id: v.id,
            greeting_id: v.greeting_id,
            message_id: Uuid::to_string( &v.message_id),
            created: v.created,
        })),
        None => Ok((HttpResponse::NoContent()).body("No content"))
    }
}

fn generate_pg_trace_context() -> PgTraceContext {
    let span = Span::current().context().span().span_context().clone();
    let trace_id = format!("{}", span.trace_id());
    let span_id = format!("{:?}", span.span_id());
    let pg_trace = PgTraceContext {
        trace_id,
        parent_span_id: span_id,
    };
    pg_trace
}

pub async fn generate_log(pool: Box<Pool<Postgres>>) -> Result<(), ApiError> {
    loop {
        inner_generate_log(pool.clone()).await?;
        sleep(Duration::from_secs(5)).await;
    }
}

#[instrument(name = "generate_log")]
async fn inner_generate_log(pool: Box<Pool<Postgres>>) -> Result<(), ApiError> {
    let pg_trace = generate_pg_trace_context();

    info!("Generating logs");
    if let Err(e) = greeting_db_api::generate_logg(&pool, pg_trace).await {
        error!("Failed to generate logg: {:?}", e);
    }
    Ok(())
}

#[derive(Debug, Display)]
pub enum ApiError {
    ApplicationError(DbError),
    NotFound(String),
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            // BadClientData(_) => StatusCode::BAD_REQUEST,
            ApplicationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            NotFound(_) => StatusCode::NOT_FOUND
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::plaintext())
            .body(self.to_string())
    }
}

impl From<DbError> for ApiError {
    fn from(value: DbError) -> Self {
        ApplicationError(value)
    }
}
