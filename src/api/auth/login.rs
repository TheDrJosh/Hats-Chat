use axum::{extract::State, Form};
use email_address::EmailAddress;
use http::{HeaderMap, StatusCode, HeaderName, HeaderValue};
use serde::Deserialize;
use sqlx::PgPool;
use tower_cookies::Cookies;

use crate::{
    api::auth::make_jwt_token, data::app_state::AppState, utils::ToServerError, LogInTemplate,
};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub async fn login(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> Result<Result<LogInTemplate, HeaderMap>, (StatusCode, String)> {
    tracing::debug!("request login for user ({}).", form.username,);

    match get_password_hash_from_username_or_email(&form.username, &state.pool)
        .await
        .server_error()?
    {
        Some((user_id, stored_password_hash)) => {
            tracing::debug!("found user ({}) id ({}). ", form.username, user_id);
            let passwords_match =
                bcrypt::verify(form.password, &stored_password_hash).server_error()?;
            if passwords_match {
                tracing::debug!("password correct: id: {}.", user_id);

                make_jwt_token(user_id, form.username, &cookies, state)
                    .await
                    .server_error()?;

                tracing::debug!("created tokens: id: {}.", user_id);

                let mut headers = HeaderMap::default();

                headers.insert(
                    HeaderName::from_static("hx-refresh"),
                    HeaderValue::from_static("true"),
                );

                Ok(Err(headers))
            } else {
                tracing::debug!(
                    "login atempt for user ({}) failed wrong password",
                    form.username
                );
                Ok(Ok(LogInTemplate::with_error(
                    "Wrong username or password".to_owned(),
                )))
            }
        }
        None => {
            tracing::debug!("no user ({}) found", form.username);

            Ok(Ok(LogInTemplate::with_error(
                "Wrong username or password".to_owned(),
            )))
        }
    }
}

async fn get_password_hash_from_username_or_email(
    username: &str,
    pool: &PgPool,
) -> anyhow::Result<Option<(i32, String)>> {
    if EmailAddress::is_valid(username) {
        Ok(sqlx::query!(
            "SELECT id, password_hash FROM users WHERE email = $1",
            username
        )
        .fetch_optional(pool)
        .await?
        .map(|rec| (rec.id, rec.password_hash)))
    } else {
        Ok(sqlx::query!(
            "SELECT id, password_hash FROM users WHERE username = $1",
            username
        )
        .fetch_optional(pool)
        .await?
        .map(|rec| (rec.id, rec.password_hash)))
    }
}
