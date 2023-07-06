mod auth;

use axum::Router;


pub fn api_routes() -> Router {
    Router::new().nest("/auth", auth::auth_routes())
}

