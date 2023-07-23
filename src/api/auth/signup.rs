use axum::{extract::State, Form};
use email_address::EmailAddress;
use http::{HeaderMap, StatusCode, HeaderName, HeaderValue};
use serde::Deserialize;
use sqlx::PgPool;
use tower_cookies::Cookies;

use crate::{data::app_state::AppState, utils::ToServerError, SignUpTemplate, activate::send_confirmation_email};

use super::make_jwt_token;

#[derive(Debug, Deserialize)]
pub struct CreateUserForm {
    username: String,
    email: String,
    password: String,
    confirm_password: String,
}

async fn email_in_database(email: &str, pool: &PgPool) -> anyhow::Result<bool> {
    let exists = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1);",
        email
    )
    .fetch_one(pool)
    .await?
    .exists
    .unwrap_or_default();
    Ok(exists)
}

pub async fn signup(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<CreateUserForm>,
) -> Result<Result<SignUpTemplate, HeaderMap>, (StatusCode, String)> {
    // check if passwords match
    if form.password != form.confirm_password {
        return Ok(Ok(SignUpTemplate::with_password_error(
            "Your  passwords must match.".to_owned(),
        )));
    }
    // check if email valid
    if !EmailAddress::is_valid(&form.email) {
        return Ok(Ok(SignUpTemplate::with_email_error(
            "Invalid Email address.".to_owned(),
        )));
    }
    // check if email in database
    if email_in_database(&form.email, &state.pool)
        .await
        .server_error()?
    {
        return Ok(Ok(SignUpTemplate::with_email_error(
            "Email already used.".to_owned(),
        )));
    }
    // chech if username in database
    if username_in_database(&form.username, &state.pool)
        .await
        .server_error()?
    {
        return Ok(Ok(SignUpTemplate::with_username_error(
            "Username already taken.".to_owned(),
        )));
    }

    let password_hashed = bcrypt::hash(form.password, bcrypt::DEFAULT_COST).server_error()?;

    let user_id = sqlx::query!(
        "INSERT INTO users(username, email, password_hash) VALUES ($1, $2, $3) RETURNING id;",
        form.username,
        form.email,
        password_hashed
    )
    .fetch_one(&state.pool)
    .await
    .server_error()?
    .id;

    send_confirmation_email(user_id, state.clone()).await.server_error()?;

    make_jwt_token(user_id, form.username, &cookies, state)
        .await
        .server_error()?;

    let mut headers = HeaderMap::default();

    headers.insert(
        HeaderName::from_static("hx-refresh"),
        HeaderValue::from_static("true"),
    );
    Ok(Err(headers))
}

async fn username_in_database(username: &str, pool: &PgPool) -> anyhow::Result<bool> {
    let exists = sqlx::query!(
        "SELECT EXISTS(SELECT username FROM users WHERE username = $1);",
        username
    )
    .fetch_one(pool)
    .await?
    .exists
    .unwrap_or_default();

    Ok(exists)
}
