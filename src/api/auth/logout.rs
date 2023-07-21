use askama_axum::IntoResponse;
use axum::extract::State;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use tower_cookies::Cookies;

use crate::{
    data::app_state::AppState,
    utils::{auth_layer::ExtractAuth, ToServerError},
};

use super::AUTH_COOKIE_NAME;

pub async fn logout(
    State(state): State<AppState>,
    cookies: Cookies,
    ExtractAuth(user_id): ExtractAuth,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let private_cookies = cookies.private(&state.cookie_key);

    match private_cookies.get(AUTH_COOKIE_NAME) {
        Some(token) => {
            sqlx::query!(
                "DELETE FROM auth_tokens WHERE user_id = $1 AND token = $2",
                user_id,
                token.value()
            )
            .execute(&state.pool)
            .await
            .server_error()?;

            private_cookies.remove(token.clone());

            let mut headers = HeaderMap::default();

            headers.insert(
                HeaderName::from_static("hx-refresh"),
                HeaderValue::from_static("true"),
            );

            Ok(headers)
        }
        None => {
            tracing::error!("schrodinger's log in for user({user_id})");
            Err((StatusCode::INTERNAL_SERVER_ERROR, String::from("schrodinger's log in")))
        }
    }
}
