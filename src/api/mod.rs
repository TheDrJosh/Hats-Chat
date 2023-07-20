pub mod auth;
pub mod chat;
pub mod account;

use axum::Router;
use http::StatusCode;

use crate::data::app_state::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::auth_routes())
        .nest("/chat", chat::chat_routes())
        .nest("/account", account::account_details_uris())
        .fallback(not_found)
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
