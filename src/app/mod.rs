use axum::response::Html;
use http::StatusCode;

use crate::{data::app_state::AppState, utils::ToServerError};

pub async fn main(state: AppState, user_id: i32) -> Result<Html<String>, StatusCode> {
    let rec = sqlx::query!(
        "SELECT username, email, display_name from users WHERE id = $1",
        user_id
    )
    .fetch_one(&state.pool)
    .await
    .server_error()?;

    Ok(page_template(&rec.username))
}

fn page_template(username: &str) -> Html<String> {
    Html(format!(
        include_str!("../../pages/index.html"),
        account_name = username,
        chat_list = "chat_list",
        chat = "chat"
    ))
}
