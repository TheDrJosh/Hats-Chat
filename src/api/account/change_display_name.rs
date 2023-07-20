use axum::{extract::State, Form};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use tower_cookies::Cookies;

use crate::{api::auth::logged_in, data::app_state::AppState, utils::ToServerError};

#[derive(serde::Deserialize)]
pub struct ChangeDisplayNameForm {
    display_name: String,
}

pub async fn change_display_name(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<ChangeDisplayNameForm>,
) -> Result<HeaderMap, StatusCode> {
    let user_id = logged_in(&state, &cookies).await.server_error()?.ok_or(StatusCode::UNAUTHORIZED)?;
    tracing::debug!("display nane change for user ({})", user_id);

    if form.display_name.is_empty() {
        tracing::debug!("bad display name from user ({})", user_id);
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query!(
        "UPDATE users SET display_name = $1 WHERE id = $2",
        form.display_name,
        user_id
    )
    .execute(&state.pool)
    .await
    .server_error()?;

    tracing::debug!("edited display name for user ({})", user_id);

    let mut headers = HeaderMap::default();

    headers.insert(
        HeaderName::from_static("hx-refresh"),
        HeaderValue::from_static("true"),
    );

    Ok(headers)
}
