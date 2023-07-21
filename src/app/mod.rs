use askama::Template;
use http::StatusCode;
use sqlx::PgPool;

use crate::{api::chat::ChatWindowInfo, data::app_state::AppState, utils::ToServerError};

use self::friend_list::FiendListInfo;

mod friend_list;
pub mod find_friend;
pub mod account;

pub async fn main(
    state: AppState,
    user_id: i32,
    recipient: Option<String>,
) -> Result<Base, (StatusCode, String)> {
    let base_info = BaseInfo::new(user_id, &state.pool).await.server_error()?;

    let chat_window_info = match recipient {
        Some(recipient) => {
            let other_user_id = sqlx::query!("SELECT id FROM users WHERE username = $1", recipient)
                .fetch_one(&state.pool)
                .await
                .server_error()?
                .id;

            Some(
                ChatWindowInfo::new(user_id, other_user_id, &state.pool)
                    .await
                    .server_error()?,
            )
        }
        None => None,
    };

    let friend_list_info = FiendListInfo::new(user_id, &state.pool)
        .await
        .server_error()?;

    let base = Base {
        base_info,
        chat_window_info,
        friend_list_info,
    };

    Ok(base)
}

#[derive(Template)]
#[template(path = "base.html")]
pub struct Base {
    pub base_info: BaseInfo,
    pub chat_window_info: Option<ChatWindowInfo>,
    pub friend_list_info: FiendListInfo,
}

pub struct BaseInfo {
    pub user_id: i32,
    pub username: String,
    pub display_name: String,
}

impl BaseInfo {
    pub async fn new(user_id: i32, pool: &PgPool) -> anyhow::Result<Self> {
        Ok(sqlx::query!(
            "SELECT username, display_name FROM users WHERE id = $1",
            user_id
        )
        .fetch_one(pool)
        .await
        .map(|rec| BaseInfo {
            user_id,
            username: rec.username.clone(),
            display_name: rec.display_name.unwrap_or(rec.username),
        })?)
    }
}
