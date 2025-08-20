use std::fmt::{Display, Formatter};
use crate::greeting::ApiError::ApplicationError;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{get, web, HttpResponse, ResponseError};
use chrono::{DateTime, Utc};
use derive_more::Display;
use greeting_db_api::greeting_query::{
    GreetingQueryRepository, GreetingQueryRepositoryImpl, LoggQueryEntity,
};
use greeting_db_api::DbError;
use log::{error, info};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use sqlx::{Pool, Postgres};
use tracing::instrument;
use utoipa::ToSchema;
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

impl Display for LoggQuery{
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
    #[schema(value_type = String, format = DateTime)]
    created: DateTime<Utc>,
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
        (status = NOT_FOUND, description = "Greetings was not found")
    )
)]
#[get("/log")]
#[instrument(name = "log")]
pub async fn list_log_entries(
    data: Data<Box<GreetingQueryRepositoryImpl>>,
    logg_query: web::Query<LoggQuery>,
) -> Result<HttpResponse, ApiError> {
    info!("Access logg {}", &logg_query);

    let query = LoggQueryEntity {
        offset: logg_query.offset,
        limit: logg_query.limit,
        direction: logg_query.direction.clone(),
    };

    let result = data.list_log_entries(query).await?;

    let logg_list = result
        .iter()
        .map(|e| LoggEntry {
            id: e.id,
            greeting_id: e.greeting_id,
            created: e.created,
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(logg_list))
}

pub async fn generate_logg(pool: Box<Pool<Postgres>>)->Result<(), ApiError> {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        info!("Generating logs");
        if let Err(e) = greeting_db_api::generate_logg(&pool).await {
            error!("Failed to generate logg: {:?}", e);
        }
    }
}

#[derive(Debug, Display)]
pub enum ApiError {
    ApplicationError(DbError),
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            // BadClientData(_) => StatusCode::BAD_REQUEST,
            ApplicationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }
}

impl From<DbError> for ApiError {
    fn from(value: DbError) -> Self {
        ApplicationError(value)
    }
}
