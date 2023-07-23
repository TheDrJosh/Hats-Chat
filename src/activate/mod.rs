use askama::Template;
use axum::{
    extract::{Path, State},
    response::Redirect,
    routing::{get, post},
    Router,
};
use http::StatusCode;
use jsonwebtoken::Header;
use lettre::{message::header::ContentType, Message, Transport};

use crate::{
    data::app_state::AppState,
    utils::{username::Username, ToServerError},
};

pub async fn send_confirmation_email(user_id: i32, state: AppState) -> anyhow::Result<()> {
    let email_addr = sqlx::query!("SELECT email FROM users WHERE id = $1", user_id)
        .fetch_one(&state.pool)
        .await?
        .email;

    let username = Username::new_from_id(user_id, &state.pool).await?;

    let confirmation_email_template = ConfirmationEmailTemplate {
        token: generate_confirmation_tokens(user_id, username.username(), state.clone()).await?,
        username,
    };

    let email = Message::builder()
        .from("Hats Chat <josh.a.roo2004@gmail.com>".parse()?)
        .to(email_addr.parse()?)
        .subject("Account Confirmation")
        .header(ContentType::TEXT_HTML)
        .body(confirmation_email_template.render()?)?;

    state.mailer.send(&email)?;

    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claim {
    sub: String,
    exp: usize,
}

async fn generate_confirmation_tokens(
    user_id: i32,
    username: String,
    state: AppState,
) -> anyhow::Result<String> {
    let claim = Claim {
        sub: username,
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claim,
        &jsonwebtoken::EncodingKey::from_base64_secret(&state.jws_key)?,
    )?;

    sqlx::query!("DELETE FROM account_activation WHERE id = $1", user_id)
        .execute(&state.pool)
        .await?;

    let utc = time::OffsetDateTime::now_utc();
    let timestamp = time::PrimitiveDateTime::new(utc.date(), utc.time());

    sqlx::query!(
        "INSERT INTO account_activation(id, token, created) VALUES ($1, $2, $3)",
        user_id,
        token,
        timestamp
    )
    .execute(&state.pool)
    .await?;

    Ok(token)
}

#[derive(Template)]
#[template(path = "email/confirmation_email.html")]
struct ConfirmationEmailTemplate {
    pub username: Username,
    pub token: String,
}

pub fn activate_routes() -> Router<AppState> {
    Router::new()
        .route("/:username/resend", post(resend))
        .route("/:username/:jwt", get(activate_account))
}

pub async fn resend(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let user_id = sqlx::query!("SELECT id FROM users WHERE username = $1", username)
        .fetch_one(&state.pool)
        .await
        .server_error()?
        .id;

    send_confirmation_email(user_id, state)
        .await
        .server_error()?;

    Ok((StatusCode::OK, String::from("sent email")))
}

pub async fn activate_account(
    Path((username, jwt)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Redirect, (StatusCode, String)> {
    tracing::debug!("trying to activate account");

    let token_user_id = sqlx::query!("SELECT id FROM account_activation WHERE token = $1", jwt)
        .fetch_one(&state.pool)
        .await
        .server_error()?
        .id;

    let token_username = sqlx::query!("SELECT username FROM users WHERE id = $1", token_user_id)
        .fetch_one(&state.pool)
        .await
        .server_error()?
        .username;

    if username != token_username {
        return Err((
            StatusCode::BAD_REQUEST,
            String::from("token not in database for that user"),
        ));
    }

    let mut validation = jsonwebtoken::Validation::default();
    validation.sub = Some(username);

    jsonwebtoken::decode::<Claim>(
        &jwt,
        &jsonwebtoken::DecodingKey::from_base64_secret(&state.jws_key).server_error()?,
        &validation,
    )
    .server_error()?;

    sqlx::query!(
        "DELETE FROM account_activation WHERE id = $1",
        token_user_id
    )
    .execute(&state.pool)
    .await
    .server_error()?;

    sqlx::query!(
        "UPDATE users SET activated = true WHERE id = $1",
        token_user_id
    )
    .execute(&state.pool)
    .await
    .server_error()?;

    Ok(Redirect::to("/"))
}
