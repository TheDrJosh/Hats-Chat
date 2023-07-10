use axum::{
    extract::{Path, State},
    routing::post,
    Form, Router,
};
use http::StatusCode;
use tower_cookies::Cookies;

use crate::{
    api::auth::logged_in,
    data::app_state::AppState,
    utils::{RowOptional, ToServerError},
};

pub fn chat_routes() -> Router<AppState> {
    Router::new().route("/:recipient", post(post_chat))
}

#[derive(serde::Deserialize)]
struct PostChatForm {
    message: String,
}

async fn post_chat(
    Path(recipient_name): Path<String>,
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<PostChatForm>,
) -> Result<StatusCode, StatusCode> {
    match logged_in(&state, &cookies).await.server_error()? {
        Some(user_id) => {
            match sqlx::query!("SELECT id FROM users WHERE username = $1", recipient_name)
                .fetch_one(&state.pool)
                .await
                .optional()
                .server_error()?
                .map(|rec| rec.id)
            {
                Some(recipient_id) => {
                    let utc = time::OffsetDateTime::now_utc();
                    let timestamp = time::PrimitiveDateTime::new(utc.date(), utc.time());

                    sqlx::query!("INSERT INTO chat_messages(sender_id, recipient_id, msg, sent_at) VALUES ($1, $2, $3, $4);",user_id, recipient_id, form.message, timestamp).execute(&state.pool).await.server_error()?;

                    Ok(StatusCode::OK)
                }
                None => Err(StatusCode::BAD_REQUEST),
            }
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
