use axum::{async_trait, extract::FromRequestParts};
use http::{request::Parts, StatusCode};
use tower_cookies::Cookies;

use crate::{api::auth::Claim, data::app_state::AppState, utils::ToServerError};

// Optional auth

pub struct OptionalExtractAuth(pub Option<i32>);

#[async_trait]
impl FromRequestParts<AppState> for OptionalExtractAuth {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(logged_in(parts, state).await?))
    }
}

// auth

pub struct ExtractAuth(pub i32);

#[async_trait]
impl FromRequestParts<AppState> for ExtractAuth {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        return Ok(Self(logged_in(parts, state).await?.ok_or((
            StatusCode::UNAUTHORIZED,
            String::from("Unauthorized"),
        ))?));
    }
}

async fn logged_in(
    parts: &mut Parts,
    state: &AppState,
) -> Result<Option<i32>, (StatusCode, String)> {
    let cookies = Cookies::from_request_parts(parts, state)
        .await
        .server_error()?;
    let private_cookies = cookies.private(&state.cookie_key);

    match private_cookies.get(crate::api::auth::AUTH_COOKIE_NAME) {
        Some(cookie_token) => {
            let user_id = sqlx::query!(
                "SELECT user_id FROM auth_tokens WHERE token = $1",
                cookie_token.value()
            )
            .fetch_optional(&state.pool)
            .await
            .server_error()?
            .map(|rec| rec.user_id);

            match user_id {
                Some(user_id) => {
                    let username =
                        sqlx::query!("SELECT username FROM users WHERE id = $1", user_id)
                            .fetch_one(&state.pool)
                            .await
                            .server_error()?
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

                    match res {
                        Ok(_) => {
                            tracing::debug!("user (id: {}) is logged in.", user_id);

                            return Ok(Some(user_id));
                        }
                        Err(e) => match e.kind() {
                            jsonwebtoken::errors::ErrorKind::ExpiredSignature
                            | jsonwebtoken::errors::ErrorKind::InvalidSubject => {
                                tracing::debug!(
                                    "user (id: {}) submited invalid jwt token.",
                                    user_id
                                );

                                private_cookies.remove(cookie_token);
                                return Ok(None);
                            }
                            _ => Err(e).server_error()?,
                        },
                    }
                }
                None => {
                    tracing::debug!("token not in database.");

                    private_cookies.remove(cookie_token);

                    return Ok(None);
                }
            }
        }
        None => {
            tracing::debug!("didn't find auth cookie.");
            return Ok(None);
        }
    }
}
