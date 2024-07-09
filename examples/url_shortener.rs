use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{header::LOCATION, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::{prelude::FromRow, Pool, Postgres};
use thiserror::Error;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

const BASE_URL: &str = "localhost:9876";

#[derive(Debug, FromRow)]
struct Url {
    id: String,
    url: String,
}

#[derive(Debug)]
struct AppState {
    db_pool: sqlx::PgPool,
}

#[derive(Deserialize, Debug)]
struct ShortenUrlRequest {
    url: String,
}

#[derive(Error, Debug)]
enum UrlShortenerError {
    #[error("Url not found")]
    NotFound(#[from] sqlx::Error),
}

#[tokio::main]
async fn main() -> Result<()> {
    let layer = tracing_subscriber::fmt::Layer::new()
        .with_writer(std::io::stdout)
        .pretty()
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let listener = tokio::net::TcpListener::bind(BASE_URL).await?;
    info!("Listening on: {}", BASE_URL);

    let db_url = "postgres://ecosystem:ecosystem@localhost:5432";
    let db_pool = sqlx::PgPool::connect(db_url).await?;
    let app_state = Arc::new(AppState { db_pool });

    let router = Router::new()
        .route("/", post(shorten_url))
        .route("/:id", get(redirect_to_url))
        .with_state(app_state);
    axum::serve(listener, router.into_make_service()).await?;
    Ok(())
}

async fn shorten_url(
    State(app_state): State<Arc<AppState>>,
    Json(request_body): Json<ShortenUrlRequest>,
) -> impl IntoResponse {
    let url = Url::new(request_body.url);
    let db_pool = app_state.db_pool.clone();
    let id = url.add_shortened_url(db_pool).await.unwrap();
    let shortened_url = format!("http://{}/{}", BASE_URL, id);
    (StatusCode::CREATED, Json(shortened_url))
}

async fn redirect_to_url(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let url = Url::get_url_by_id(&id, state.db_pool.clone()).await;
    match url {
        Ok(url) => {
            let mut headers = HeaderMap::new();
            headers.insert(LOCATION, url.parse().unwrap());
            (StatusCode::FOUND, headers).into_response()
        }
        Err(e) => e.into_response(),
    }
}

impl Url {
    fn new(url: String) -> Self {
        Self {
            url,
            id: "temp".to_string(),
        }
    }

    async fn add_shortened_url(&self, db: Pool<Postgres>) -> Result<String> {
        let id = nanoid::nanoid!(6);

        // if the id already exists, generate a new one
        let mut id = id;
        while (sqlx::query("SELECT * FROM urls WHERE id = $1")
            .bind(&id)
            .fetch_one(&db)
            .await)
            .is_ok()
        {
            id = nanoid::nanoid!(6);
        }

        let url: Url = sqlx::query_as(
            r#"
            INSERT INTO urls (id, url)
            VALUES ($1, $2)
            ON CONFLICT(url) DO UPDATE SET url = EXCLUDED.url RETURNING *"#,
        )
        .bind(&id)
        .bind(&self.url)
        .fetch_one(&db)
        .await?;

        Ok(url.id)
    }

    async fn get_url_by_id(id: &str, db: Pool<Postgres>) -> Result<String, UrlShortenerError> {
        let url: Url = sqlx::query_as("SELECT * FROM urls WHERE id = $1")
            .bind(id)
            .fetch_one(&db)
            .await?;
        Ok(url.url)
    }
}

impl IntoResponse for UrlShortenerError {
    fn into_response(self) -> axum::response::Response {
        let headers = HeaderMap::new();
        match self {
            UrlShortenerError::NotFound(_) => (StatusCode::NOT_FOUND, headers),
        }
        .into_response()
    }
}
