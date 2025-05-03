use crate::greeting::ApiError::ApplicationError;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{get, web, HttpResponse, ResponseError};
use chrono::{DateTime, Utc};
use derive_more::Display;
use log::{error, info};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::query::Query;
use sqlx::{query, Execute, Executor, PgPool, Pool, Postgres, QueryBuilder, Row};
use std::time::Duration;
use tracing_subscriber::fmt::format;
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
    path = "/logs",
    params(
        ("logg_query" = LoggQuery, Query,)
    ),
    responses(
        (status = 200, description = "Greetings", body = LoggEntry),
        (status = NOT_FOUND, description = "Greetings was not found")
    )
)]
#[get("/logs")]
pub async fn list_log_entries(
    data: Data<Box<Pool<Postgres>>>,
    logg_query: web::Query<LoggQuery>,
) -> Result<HttpResponse, ApiError> {
    let direction = SqlDirection::value_of(&logg_query.direction);

    let mut logg_sql: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT id, greeting_id, opprettet FROM LOGG");

    logg_sql.push(format!(" WHERE id {} ", direction.operator));
    logg_sql.push_bind(logg_query.offset );
    logg_sql.push(format!(" ORDER BY id {}", direction.order));
    logg_sql.push(" LIMIT ");
    logg_sql.push_bind(logg_query.limit);

    let r = data
        .fetch_all(logg_sql.build())
        .await
        .map(|res| res.iter().map(|v| 
            LoggEntry { id: v.get(0) , greeting_id: v.get(1), created: v.get(2) }).collect::<Vec<_>>()
        )
        .map_err(|e| -> ApiError {
            error!("{}", e);
            ApplicationError(e)
        })?;
    

    Ok(HttpResponse::Ok().json(r))
}

struct SqlDirection {
    order: String,
    operator: String,
}

impl SqlDirection {
    fn value_of(direction: &str) -> SqlDirection {
        match direction {
            "forward" => SqlDirection {
                order: String::from("ASC"),
                operator: String::from(">="),
            },
            "backward" => SqlDirection {
                order: String::from("DESC"),
                operator: String::from("<="),
            },
            _ => panic!("Invalid direction"),
        }
    }
}

#[derive(Debug, Display)]
pub enum ApiError {
    ApplicationError(sqlx::Error),
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

impl From<sqlx::Error> for ApiError {
    fn from(value: sqlx::Error) -> Self {
        ApplicationError(value)
    }
}

pub async fn generate_logg(pool: Data<Box<Pool<sqlx::Postgres>>>) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        match pool.begin().await {
            Err(e) => error!("{}", e),
            Ok(mut transaction) => {
                sqlx::query(
                    "do
                        $$
                            begin
                                perform public.generate_logg();
                            end
                        $$;",
                )
                    .execute(&mut *transaction)
                    .await
                    .expect("Failed executing statement");
                info!("Generating log");
                transaction.commit().await.expect("");
            }
        }
    }
}
