use axum::{routing::post, Router};
use cookie::time::{Duration, OffsetDateTime};
use jsonwebtoken::Header;
use tower_cookies::{Cookie, Cookies};

use crate::data::app_state::AppState;

mod login;
mod logout;
mod signup;

pub const AUTH_COOKIE_NAME: &str = "web_chat_app_token";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claim {
    sub: String,
    exp: usize,
}

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/user/create", post(signup::signup))
        .route("/user/login", post(login::login))
        .route("/user/logout", post(logout::logout))
}

pub async fn make_jwt_token(
    user_id: i32,
    username: String,
    cookies: &Cookies,
    state: AppState,
) -> anyhow::Result<String> {
    let claim = Claim {
        sub: username,
        exp: (chrono::Utc::now() + chrono::Duration::days(7)).timestamp() as usize,
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claim,
        &jsonwebtoken::EncodingKey::from_base64_secret(&state.jws_key)?,
    )?;

    sqlx::query!(
        "INSERT INTO auth_tokens(token, user_id) VALUES ($1, $2);",
        &token,
        user_id,
    )
    .execute(&state.pool)
    .await?;

    tracing::debug!("inserted token into database (user id: {}).", user_id);

    cookies.private(&state.cookie_key).add(
        Cookie::build(AUTH_COOKIE_NAME, token.clone())
            .path("/")
            .expires(OffsetDateTime::now_utc().checked_add(Duration::days(7)))
            .finish(),
    );

    Ok(token)
}
