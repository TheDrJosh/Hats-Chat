use axum::extract::State;
use http::StatusCode;
use tower_cookies::Cookies;

use crate::data::app_state::AppState;

pub async fn logout(State(state): State<AppState>, cookies: Cookies) -> StatusCode {
    let private_cookies = cookies.private(&state.cookie_key);

    match private_cookies.get("chat-web-app") {
        Some(token) => {
            private_cookies.remove(token);

            StatusCode::ACCEPTED
        }
        None => StatusCode::UNAUTHORIZED,
    }
}
