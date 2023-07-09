use crate::{login_partial, utils::ToServerError};
use axum::{extract::State, response::Html, routing::post, Form, Router};
use email_address::EmailAddress;
use http::StatusCode;
use jsonwebtoken::Header;
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claim {
    sub: String,
    exp: usize,
}

pub fn auth_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/user/create", post(create))
        .route("/user/login", post(login))
        .route("/user/logout", post(logout))
        .with_state(pool)
}

#[derive(Debug, Deserialize)]
struct CreateUserForm {
    username: String,
    email: String,
    password: String,
    confirm_password: String,
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

async fn create(
    State(pool): State<PgPool>,
    Form(form): Form<CreateUserForm>,
) -> Result<Html<String>, StatusCode> {
    // check if passwords match
    if form.password != form.confirm_password {
        return Ok(Html(crate::signup_partial(
            "",
            "",
            "Your  passwords must match.",
        )));
    }
    // check if email valid
    if !EmailAddress::is_valid(&form.email) {
        return Ok(Html(crate::signup_partial(
            "",
            "Invalid Email address.",
            "",
        )));
    }
    // check if email in database
    if email_in_database(&form.email, &pool).await.server_eror()? {
        return Ok(Html(crate::signup_partial("", "Email already used.", "")));
    }
    // chech if username in database
    if username_in_database(&form.username, &pool)
        .await
        .server_eror()?
    {
        return Ok(Html(crate::signup_partial(
            "Username already taken.",
            "",
            "",
        )));
    }

    let password_hashed = bcrypt::hash(form.password, bcrypt::DEFAULT_COST).server_eror()?;

    let user_id = sqlx::query!(
        "INSERT INTO users(username, email, password_hash) VALUES ($1, $2, $3) RETURNING id;",
        form.username,
        form.email,
        password_hashed
    )
    .fetch_one(&pool)
    .await
    .server_eror()?
    .id;

    let token = make_jwt_token(form.username).server_eror()?;

    sqlx::query!(
        "INSERT INTO auth_tokens(token, user_id) VALUES ($1, $2);",
        token,
        user_id,
    )
    .execute(&pool)
    .await
    .server_eror()?;

    Ok(Html(format!(
        "It worked! id: {}, token: {}",
        user_id, token
    )))
}

fn make_jwt_token(username: String) -> anyhow::Result<String> {
    let claim = Claim {
        sub: username,
        exp: (chrono::Utc::now() + chrono::Duration::minutes(5)).timestamp() as usize,
    };

    let secret = dotenvy::var("JWS_SECRET")?;
    Ok(jsonwebtoken::encode(
        &Header::default(),
        &claim,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login(
    State(pool): State<PgPool>,
    Form(form): Form<LoginForm>,
) -> Result<Html<String>, StatusCode> {
    tracing::debug!(
        "request login for user ({}) with password ({}).",
        form.username,
        form.password
    );

    match get_password_hash_from_username_or_email(&form.username, &pool)
        .await
        .server_eror()?
    {
        Some((user_id, stored_password_hash)) => {
            tracing::debug!("found user ({}) id ({}). ", form.username, user_id);
            let passwords_match = bcrypt::verify(form.password, &stored_password_hash)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if passwords_match {
                // TODO! redirect to / with credentails

                let token = make_jwt_token(form.username).server_eror()?;

                Ok(Html(format!(
                    "It worked: id: {}, token: {}",
                    user_id, token
                )))
            } else {
                tracing::debug!(
                    "login atempt for user ({}) failed wrong password",
                    form.username
                );
                Ok(Html(login_partial("Wrong username or password")))
            }
        }
        None => {
            tracing::debug!("no user ({}) found", form.username);

            Ok(Html(login_partial("Wrong username or password")))
        }
    }
}

async fn get_password_hash_from_username_or_email(
    username: &str,
    pool: &PgPool,
) -> anyhow::Result<Option<(i32, String)>> {
    if EmailAddress::is_valid(username) {
        let rec = sqlx::query!(
            "SELECT id, password_hash FROM users WHERE email = $1",
            username
        )
        .fetch_one(pool)
        .await;

        match rec {
            Ok(rec) => Ok(Some((rec.id, rec.password_hash))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e)?,
        }
    } else {
        let rec = sqlx::query!(
            "SELECT id, password_hash FROM users WHERE username = $1",
            username
        )
        .fetch_one(pool)
        .await;

        match rec {
            Ok(rec) => Ok(Some((rec.id, rec.password_hash))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e)?,
        }
    }
}

async fn logout(State(pool): State<PgPool>) -> () {}
