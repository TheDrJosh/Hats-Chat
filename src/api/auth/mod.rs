use axum::{routing::post, Router};
use jsonwebtoken::{Header, TokenData};
use tower_cookies::{Cookie, Cookies};

use crate::data::app_state::AppState;

mod login;
mod logout;
mod signup;

const COOKIE_NAME: &'static str = "web_chat_app_token";

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
        exp: (chrono::Utc::now() + chrono::Duration::minutes(5)).timestamp() as usize,
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claim,
        &jsonwebtoken::EncodingKey::from_secret(state.jws_key.as_bytes()),
    )?;

    let res = sqlx::query!(
        "INSERT INTO auth_tokens(token, user_id) VALUES ($1, $2);",
        &token,
        user_id,
    )
    .execute(&state.pool)
    .await?;

    tracing::debug!("inserted token into database (user id: {}).", user_id);

    cookies
        .private(&state.cookie_key)
        .add(Cookie::build(COOKIE_NAME, token.clone()).path("/").finish());

    Ok(token)
}

pub async fn logged_in(state: &AppState, cookies: &Cookies) -> anyhow::Result<Option<i32>> {
    let private_cookies = cookies.private(&state.cookie_key);

    match private_cookies.get(COOKIE_NAME) {
        Some(cookie_token) => {
            tracing::debug!("found token cookie.");

            let user_record_result = sqlx::query!(
                "SELECT user_id FROM auth_tokens WHERE token = $1",
                cookie_token.value()
            )
            .fetch_one(&state.pool)
            .await;

            let user_record = match user_record_result {
                Ok(rec) => Ok(Some(rec)),
                Err(sqlx::Error::RowNotFound) => Ok(None),
                Err(e) => Err(e),
            }?
            .map(|rec| rec.user_id);

            match user_record {
                Some(user_id) => {
                    let username =
                        sqlx::query!("SELECT username FROM users WHERE id = $1", user_id)
                            .fetch_one(&state.pool)
                            .await?
                            .username;

                    tracing::debug!(
                        "found user in database with token. id: {}, username: {}.",
                        user_id,
                        &username
                    );

                    let mut validation = jsonwebtoken::Validation::default();
                    validation.sub = Some(username);

                    let res = jsonwebtoken::decode::<Claim>(
                        cookie_token.value(),
                        &jsonwebtoken::DecodingKey::from_secret(state.jws_key.as_bytes()),
                        &validation,
                    );
                    use jsonwebtoken::errors::ErrorKind;
                    match res {
                        Ok(_) => {
                            tracing::debug!("user (id: {}) is logged in.", user_id);

                            Ok(Some(user_id))
                        }
                        Err(e) => match e.kind() {
                            ErrorKind::ExpiredSignature | ErrorKind::InvalidSubject => {
                                tracing::debug!(
                                    "user (id: {}) submited invalid jwt token.",
                                    user_id
                                );

                                private_cookies.remove(cookie_token);
                                Ok(None)
                            }
                            _ => Err(e)?,
                        },
                    }
                }
                None => {
                    tracing::debug!("token not in database.");

                    private_cookies.remove(cookie_token);

                    Ok(None)
                }
            }
        }
        None => {
            tracing::debug!("didn't find token cookie.");

            Ok(None)
        }
    }
}
