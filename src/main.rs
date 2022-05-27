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

use axum::{extract::Extension, http::StatusCode, Json, Router, routing::get};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::{debug,Level};
use tracing_subscriber::FmtSubscriber;

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
        .route("/entries", get(get_blogs))
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

/// Utility function for mapping any error into a `500 Internal Server Error` response.
fn internal_error<E>(err: E) -> (StatusCode, String)
    where
        E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[derive(Serialize, Deserialize, Clone, Debug, sqlx::FromRow)]
struct BlogEntry {
    created: DateTime<Utc>,
    title: String,
    author: String,
    text: String,
}