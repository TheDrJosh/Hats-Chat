mod auth;

use axum::Router;


pub fn api_routes() -> Router {
    Router::new().nest("/api", auth::auth_routes())
}

