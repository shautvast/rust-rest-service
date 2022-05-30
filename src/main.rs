//! Example of application using <https://github.com/launchbadge/sqlx>
//!
//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p example-sqlx-postgres
//! ```
//!
//! Test with curl:
//!
//! ```not_rust
//! curl 127.0.0.1:3000
//! curl -X POST 127.0.0.1:3000
//! ```

use std::{net::SocketAddr, time::Duration};

use axum::{http::StatusCode, Json, response::{IntoResponse, Response}, Router, routing::get, BoxError};
use axum::extract::{Extension, FromRequest, RequestParts, Json as ExtractJson};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::{debug, Level};
use tracing_subscriber::FmtSubscriber;
use thiserror::Error;
use validator::Validate;
use async_trait::async_trait;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let db_connection_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:1234@localhost".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");

    let create_database_sql = include_str!("create_database.sql");
    let statements = create_database_sql.split(";");
    for statement in statements {
        sqlx::query(statement).execute(&pool).await.expect("error running script");
    }

    let app = Router::new()
        .route("/entries", get(get_blogs).post(add_blog))
        .layer(Extension(pool));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_blogs(Extension(pool): Extension<PgPool>) -> Result<Json<Vec<BlogEntry>>, (StatusCode, String)> {
    debug!("handling BlogEntries request");

    sqlx::query_as("select created, title, author, text from blog_entry")
        .fetch_all(&pool)
        .await
        .map(|r| Json(r))
        .map_err(internal_error)
}

async fn add_blog(Extension(pool): Extension<PgPool>, ValidatedJson(blog): ValidatedJson<BlogEntry>) -> Result<Json<String>, (StatusCode, String)> {
    debug!("handling BlogEntries request");

    sqlx::query("insert into blog_entry (created, title, author, text) values ($1, $2, $3, $4)")
        .bind(blog.created)
        .bind(blog.title)
        .bind(blog.author)
        .bind(blog.text)
        .execute(&pool)
        .await
        .map_err(internal_error)?;

    Ok(Json("created".to_owned()))
}

/// Utility function for mapping any error into a `500 Internal Server Error` response.
fn internal_error<E>(err: E) -> (StatusCode, String)
    where
        E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::FromRow, Validate)]
struct BlogEntry {
    created: DateTime<Utc>,
    #[validate(length(min = 10, max = 100, message = "Title length must be between 10 and 100"))]
    title: String,
    #[validate(email(message = "author must be a valid email address"))]
    author: String,
    #[validate(length(min = 10, message = "text length must be at least 10"))]
    text: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, B> FromRequest<B> for ValidatedJson<T>
    where
        T: DeserializeOwned + Validate,
        B: http_body::Body + Send,
        B::Data: Send,
        B::Error: Into<BoxError>,
{
    type Rejection = ServerError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let ExtractJson(value) = ExtractJson::<T>::from_request(req).await?;
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}


#[derive(Debug, Error)]
pub enum ServerError {
    #[error(transparent)]
    ValidationError(#[from] validator::ValidationErrors),

    #[error(transparent)]
    AxumFormRejection(#[from] axum::extract::rejection::JsonRejection),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        match self {
            ServerError::ValidationError(_) => {
                let message = format!("Input validation error: [{:?}]", self).replace('\n', ", ");
                (StatusCode::BAD_REQUEST, message)
            }
            ServerError::AxumFormRejection(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        }
            .into_response()
    }
}