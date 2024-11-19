use sqlx::migrate::MigrateError;
use sqlx::Pool;
use sqlx::postgres::PgPoolOptions;

#[derive(Debug)]
pub struct RepoError {
    pub error_message: String,
}

impl From<sqlx::Error> for RepoError {
    fn from(value: sqlx::Error) -> Self {
        RepoError { error_message: value.to_string() }
    }
}


pub async fn init_db(db_url: String) -> Result<Pool<sqlx::Postgres>, RepoError> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&*db_url).await?;

    Ok(pool)
}

