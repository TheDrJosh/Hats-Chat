use askama_axum::IntoResponse;
use axum::extract::State;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use tower_cookies::Cookies;

use crate::{data::app_state::AppState, utils::ToServerError};

use super::{logged_in, COOKIE_NAME};

pub async fn logout(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<impl IntoResponse, StatusCode> {
    let private_cookies = cookies.private(&state.cookie_key);

    let user_id = logged_in(&state, &cookies).await.server_error()?;

    match user_id {
        Some(user_id) => match private_cookies.get(COOKIE_NAME) {
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
            None => Err(StatusCode::UNAUTHORIZED),
        },
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
