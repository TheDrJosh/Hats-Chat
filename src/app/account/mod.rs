use askama::Template;
use axum::extract::{Path, State};
use http::StatusCode;
use sqlx::PgPool;

use crate::{
    data::app_state::AppState,
    utils::{username::Username, ToServerError, auth_layer::ExtractOptionalActivatedAuth},
};

use self::account_viewer::{account_viewer_page, AccountViewerTemplate};

mod account_viewer;

pub async fn account_route(
    Path(account_username): Path<String>,
    State(state): State<AppState>,
    ExtractOptionalActivatedAuth(user_id): ExtractOptionalActivatedAuth,
) -> Result<Result<EditableAccountTemplate, AccountViewerTemplate>, (StatusCode, String)> {

    match user_id {
        Some(user_id) => {
            let username = sqlx::query!("SELECT username FROM users WHERE id = $1", user_id)
                .fetch_one(&state.pool)
                .await
                .server_error()?
                .username;

            if username == account_username {
                Ok(Ok(editable_account_page(user_id, &state.pool).await?))
            } else {
                Ok(Err(account_viewer_page().await?))
            }
        }
        None => Ok(Err(account_viewer_page().await?)),
    }
}

async fn editable_account_page(
    user_id: i32,
    pool: &PgPool,
) -> Result<EditableAccountTemplate, (StatusCode, String)> {
    Ok(EditableAccountTemplate {
        username: Username::new_from_id(user_id, pool).await.server_error()?,
    })
}

#[derive(Template)]
#[template(path = "editable_account.html")]
pub struct EditableAccountTemplate {
    username: Username,
}
