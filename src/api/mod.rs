mod auth;

use axum::Router;
use http::StatusCode;
use sqlx::PgPool;

pub fn api_routes(pool: PgPool) -> Router {
    Router::new()
        .nest("/auth", auth::auth_routes(pool))
        .fallback(not_found)
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
