use askama::Template;
use axum::{extract::State, Form};
use http::StatusCode;
use tower_cookies::Cookies;

use crate::{
    api::auth::logged_in,
    data::app_state::AppState,
    utils::{username::Username, ToServerError},
};

pub async fn find_friend_modal() -> FindFriendModalTemplate {
    FindFriendModalTemplate
}

#[derive(Template)]
#[template(path = "components/find_friend_modal.html")]
pub struct FindFriendModalTemplate;

#[derive(serde::Deserialize)]
pub struct FindFriendForm {
    search: String,
}

pub async fn find_friend_list(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<FindFriendForm>,
) -> Result<FindFriendListTemplate, StatusCode> {
    let search = form.search;

    let user_id = logged_in(&state, &cookies)
        .await
        .server_error()?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    tracing::debug!("search from user({}) for friend with: {}", user_id, search);

    let mut name_list = sqlx::query!(
        "SELECT username, display_name FROM users WHERE SUBSTRING(username for $2) = $1 AND id != $3 LIMIT 100",
        search,
        search.len() as i32,
        user_id
    )
    .fetch_all(&state.pool)
    .await
    .server_error()?
    .into_iter()
    .map(|rec| Username::new(rec.username, rec.display_name))
    .collect::<Vec<_>>();

    for _ in 0..30 {
        name_list.push(name_list[0].clone())
    }

    Ok(FindFriendListTemplate { name_list })
}

#[derive(Template)]
#[template(path = "components/find_friend_list.html")]
pub struct FindFriendListTemplate {
    pub name_list: Vec<Username>,
}
