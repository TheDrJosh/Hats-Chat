use axum::extract::State;
use http::StatusCode;
use tower_cookies::Cookies;

use crate::{data::app_state::AppState, utils::ToServerError};

use super::{logged_in, COOKIE_NAME};

pub async fn logout(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<StatusCode, StatusCode> {
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

                //TODO send need to refresh
                Ok(StatusCode::ACCEPTED)
            }
            None => Ok(StatusCode::UNAUTHORIZED),
        },
        None => Ok(StatusCode::UNAUTHORIZED),
    }
}
