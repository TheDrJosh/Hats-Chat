use axum::{extract::State, Form};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};

use crate::{data::app_state::AppState, utils::{ToServerError, auth_layer::ExtractActivatedAuth}};

#[derive(serde::Deserialize)]
pub struct ChangeDisplayNameForm {
    display_name: String,
}

pub async fn change_display_name(
    State(state): State<AppState>,
    ExtractActivatedAuth(user_id): ExtractActivatedAuth,
    Form(form): Form<ChangeDisplayNameForm>,
) -> Result<HeaderMap, (StatusCode, String)> {
    tracing::debug!("display nane change for user ({})", user_id);

    if form.display_name.is_empty() {
        tracing::debug!("bad display name from user ({})", user_id);
        return Err((StatusCode::BAD_REQUEST, String::from("Bad Request")));
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
