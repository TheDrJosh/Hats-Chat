use std::{net::SocketAddr, sync::Arc};

use app::Base;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use data::app_state::AppState;
use http::{header, HeaderMap, HeaderValue, StatusCode};
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use tokio::sync::watch;
use tower_cookies::{CookieManagerLayer, Cookies, Key};
use tower_http::services::ServeDir;
use tracing_subscriber::prelude::*;
use utils::{auth_layer::ExtractOptionalAuth, username::Username, ToServerError};

use crate::{
    app::find_friend::{find_friend_list, find_friend_modal},
    data::app_state::AppStateInner, activate::activate_routes,
};

mod api;
mod app;
mod data;
mod activate;
mod utils;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "web_chat_app=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    if let Err(error) = dotenvy::dotenv() {
        tracing::error!("Failed to load .env file with error ({error})");
        return;
    }

    let pool = match data::database_init().await {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!("Failed to initalize database with error ({error})");
            return;
        }
    };

    if let Err(error) = data::init_tables(&pool).await {
        tracing::error!("Failed to initalize tables with error ({error})");
        return;
    };

    let (sender, _) = watch::channel((-1, -1));

    let cookie_key_master = match dotenvy::var("COOKIE_KEY") {
        Ok(cookie_key_text) => match hex::decode(cookie_key_text) {
            Ok(cookie_key_master) => cookie_key_master,
            Err(error) => {
                tracing::error!("Failed to decode cookie key with error ({error})");
                return;
            }
        },
        Err(error) => match error {
            dotenvy::Error::EnvVar(error) => match error {
                std::env::VarError::NotPresent => {
                    let key = Key::generate();

                    tracing::warn!(
                        "cookie key not specified gennerated new key: {}",
                        hex::encode(Key::generate().master())
                    );

                    Vec::from(key.master())
                }
                std::env::VarError::NotUnicode(_) => {
                    tracing::error!("Failed to get cookie key from .env with error non unicode");
                    return;
                }
            },
            _ => {
                tracing::error!("Failed to get cookie key from .env with error ({error})");
                return;
            }
        },
    };

    let jws_key = match dotenvy::var("JWS_SECRET") {
        Ok(jws_key) => jws_key,
        Err(error) => {
            tracing::error!("Failed to get jws key from .env with error ({error})");
            return;
        }
    };

    let email_username = match dotenvy::var("EMAIL_USERNAME") {
        Ok(email_username) => email_username,
        Err(error) => {
            tracing::error!("Failed to get email username from .env with error ({error})");
            return;
        }
    };

    let email_password = match dotenvy::var("EMAIL_PASSWORD") {
        Ok(email_password) => email_password,
        Err(error) => {
            tracing::error!("Failed to get email username from .env with error ({error})");
            return;
        }
    };

    let email_credentials = Credentials::new(email_username, email_password);

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(email_credentials)
        .build();

    let app_state = Arc::new(AppStateInner {
        pool,
        jws_key,
        cookie_key: Key::from(&cookie_key_master),
        message_sent: sender,
        mailer,
    });

    let app = Router::new()
        .route("/", get(handler))
        .route("/chat/:recipient", get(handler_chat))
        .route("/login", get(login))
        .route("/signup", get(signup))
        .nest("/api", api::api_routes())
        .route("/profile_pictures/:username", get(profile_pictures))
        .nest_service("/assets", ServeDir::new("assets/"))
        .route("/inner/modal/list", post(find_friend_list))
        .route("/account/:username", get(app::account::account_route))
        .nest("/confirm", activate_routes())
        .fallback(not_found)
        .with_state(app_state)
        .layer(CookieManagerLayer::new())
        .route("/inner/empty", get(empty))
        .route("/inner/modal", get(find_friend_modal));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    if let Err(error) = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
    {
        tracing::error!("Failed to launch server with error ({error})");
    }
}

async fn handler(
    State(state): State<AppState>,
    ExtractOptionalAuth(user_id): ExtractOptionalAuth,
) -> Result<Result<Base, Result<LandingPageTemplate, UnactivatedTemplate>>, (StatusCode, String)> {
    match user_id {
        Some((user_id, activated)) => {
            if activated {
                Ok(Ok(app::main(state, user_id, None).await?))
            } else {
                let username = Username::new_from_id(user_id, &state.pool)
                    .await
                    .server_error()?;
                Ok(Err(Err(UnactivatedTemplate { username })))
            }
        }
        None => Ok(Err(Ok(LandingPageTemplate))),
    }
}

#[derive(Template)]
#[template(path = "unactivated.html")]
struct UnactivatedTemplate {
    username: Username,
}

#[derive(Template)]
#[template(path = "landing_page.html")]
struct LandingPageTemplate;

async fn handler_chat(
    Path(recipient): Path<String>,
    State(state): State<AppState>,
    ExtractOptionalAuth(user_id): ExtractOptionalAuth,
) -> Result<Result<impl IntoResponse, Redirect>, (StatusCode, String)> {
    tracing::debug!("handle chat");
    match user_id {
        Some((user_id, _)) => Ok(Ok(app::main(state, user_id, Some(recipient)).await?)),
        None => Ok(Err(Redirect::to("/"))),
    }
}

async fn login(
    headers: HeaderMap,
    ExtractOptionalAuth(user_id): ExtractOptionalAuth,
) -> Result<Result<LogInTemplate, Redirect>, (StatusCode, String)> {
    if headers
        .get("HX-Request")
        .is_some_and(|header| header == "true")
    {
        Ok(Ok(LogInTemplate::default()))
    } else {
        if user_id.is_some() {
            return Ok(Err(Redirect::to("/")));
        }
        Ok(Ok(LogInTemplate::default()))
    }
}

#[derive(Template, Default)]
#[template(path = "auth/log-in.html")]
pub struct LogInTemplate {
    error: Option<String>,
}

impl LogInTemplate {
    pub fn with_error(error: String) -> Self {
        Self { error: Some(error) }
    }
}

async fn signup(
    headers: HeaderMap,
    ExtractOptionalAuth(user_id): ExtractOptionalAuth,
) -> Result<Result<SignUpTemplate, Redirect>, (StatusCode, String)> {
    if headers
        .get("HX-Request")
        .is_some_and(|header| header == "true")
    {
        Ok(Ok(SignUpTemplate::default()))
    } else if user_id.is_some() {
        Ok(Err(Redirect::to("/")))
    } else {
        Ok(Ok(SignUpTemplate::default()))
    }
}

#[derive(Template, Default)]
#[template(path = "auth/sign-up.html")]
pub struct SignUpTemplate {
    username_error: Option<String>,
    email_error: Option<String>,
    password_error: Option<String>,
}
impl SignUpTemplate {
    pub fn with_username_error(error: String) -> Self {
        Self {
            username_error: Some(error),
            email_error: None,
            password_error: None,
        }
    }
    pub fn with_email_error(error: String) -> Self {
        Self {
            username_error: None,
            email_error: Some(error),
            password_error: None,
        }
    }
    pub fn with_password_error(error: String) -> Self {
        Self {
            username_error: None,
            email_error: None,
            password_error: Some(error),
        }
    }
}

async fn not_found(
    State(_state): State<AppState>,
    _headers: HeaderMap,
    _cookies: Cookies,
) -> NotFoundTemplate {
    NotFoundTemplate
}

#[derive(Template)]
#[template(path = "not_found.html")]
struct NotFoundTemplate;

async fn profile_pictures(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<Result<impl IntoResponse, Redirect>, (StatusCode, String)> {
    let picture = sqlx::query!(
        "SELECT profile_picture FROM users WHERE username = $1",
        username
    )
    .fetch_optional(&state.pool)
    .await
    .server_error()?
    .map(|rec| rec.profile_picture);

    match picture {
        Some(Some(picture)) => {
            let body = picture;

            let mut headers = HeaderMap::new();

            headers.append(
                header::CONTENT_TYPE,
                HeaderValue::from_static("image/avif;"),
            );
            headers.append(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment;"),
            );

            Ok(Ok((headers, body)))
        }
        Some(None) => Ok(Err(Redirect::to("/assets/default_profile.avif"))),
        None => Err((
            StatusCode::NOT_FOUND,
            String::from("Profile Picture Not Found"),
        )),
    }
}

async fn empty() -> impl IntoResponse {
    StatusCode::OK
}
