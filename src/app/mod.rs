use http::StatusCode;
use askama::Template;

use crate::{data::app_state::AppState, utils::ToServerError};

pub async fn main(state: AppState, user_id: i32, recipient: Option<String>) -> Result<Base, StatusCode> {
    let rec = sqlx::query!(
        "SELECT username, email, display_name from users WHERE id = $1",
        user_id
    )
    .fetch_one(&state.pool)
    .await
    .server_error()?;

    Ok(Base)
}

#[derive(Template)]
#[template(path = "base.html")]
pub struct Base;


