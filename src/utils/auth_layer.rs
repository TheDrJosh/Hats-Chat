use axum::{async_trait, extract::FromRequestParts};
use http::{request::Parts, StatusCode};
use tower_cookies::Cookies;

use crate::{api::auth::Claim, data::app_state::AppState, utils::ToServerError};

pub struct ExtractOptionalActivatedAuth(pub Option<i32>);

#[async_trait]
impl FromRequestParts<AppState> for ExtractOptionalActivatedAuth {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match logged_in(parts, state).await? {
            Some((user_id, activated)) => {
                if activated {
                    Ok(Self(Some(user_id)))
                } else {
                    Err((StatusCode::UNAUTHORIZED, String::from("activated")))
                }
            }
            None => Ok(Self(None)),
        }
    }
}

pub struct ExtractOptionalAuth(pub Option<(i32, bool)>);

#[async_trait]
impl FromRequestParts<AppState> for ExtractOptionalAuth {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(logged_in(parts, state).await?))
    }
}

pub struct ExtractActivatedAuth(pub i32);

#[async_trait]
impl FromRequestParts<AppState> for ExtractActivatedAuth {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let (user_id, activated) = logged_in(parts, state)
            .await?
            .ok_or((StatusCode::UNAUTHORIZED, String::from("Unauthorized")))?;

        if activated {
            Ok(Self(user_id))
        } else {
            Err((StatusCode::UNAUTHORIZED, String::from("activated")))
        }
    }
}

pub struct ExtractAuth(pub i32, pub bool);

#[async_trait]
impl FromRequestParts<AppState> for ExtractAuth {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let (user_id, activated) = logged_in(parts, state)
            .await?
            .ok_or((StatusCode::UNAUTHORIZED, String::from("unauthorized")))?;

        if activated {
            Ok(Self(user_id, activated))
        } else {
            Err((StatusCode::UNAUTHORIZED, String::from("unactivated")))
        }
    }
}

pub async fn logged_in(
    parts: &mut Parts,
    state: &AppState,
) -> Result<Option<(i32, bool)>, (StatusCode, String)> {
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
                    let (username, activated) = sqlx::query!(
                        "SELECT username, activated FROM users WHERE id = $1",
                        user_id
                    )
                    .fetch_one(&state.pool)
                    .await
                    .map(|rec| (rec.username, rec.activated.unwrap_or_default()))
                    .server_error()?;

                    tracing::debug!(
                        "found user in database with token. id: {}, username: {}.",
                        user_id,
                        &username
                    );

                    let mut validation = jsonwebtoken::Validation::default();
                    validation.sub = Some(username);

                    let res = jsonwebtoken::decode::<Claim>(
                        cookie_token.value(),
                        &jsonwebtoken::DecodingKey::from_base64_secret(&state.jws_key)
                            .server_error()?,
                        &validation,
                    );

                    match res {
                        Ok(_) => {
                            tracing::debug!("user (id: {}) is logged in.", user_id);

                            Ok(Some((user_id, activated)))
                        }
                        Err(e) => match e.kind() {
                            jsonwebtoken::errors::ErrorKind::ExpiredSignature
                            | jsonwebtoken::errors::ErrorKind::InvalidSubject
                            | jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                                tracing::debug!(
                                    "user (id: {}) submited invalid jwt token.",
                                    user_id
                                );

                                private_cookies.remove(cookie_token);
                                Ok(None)
                            }
                            _ => Err(e).server_error()?,
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
            tracing::debug!("didn't find auth cookie.");
            Ok(None)
        }
    }
}
