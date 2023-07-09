use axum::response::Html;
use http::StatusCode;

use crate::{data::app_state::AppState, utils::ToServerError};


pub async fn main(state: AppState, user_id: i32)  -> Result<Html<String>, StatusCode> {
    let rec = sqlx::query!("SELECT * from users WHERE id = $1", user_id).fetch_one(&state.pool).await.server_error()?;
    let username = rec.username;
    let email = rec.email;
    let display_name = rec.display_name;
    let password_hash = rec.password_hash;

    Ok(Html(format!("used_id: {user_id}\nusername: {username}\nemail: {email}\ndisplay_name: {display_name:?}\npassword_hash: {password_hash}")))
}