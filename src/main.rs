use std::{net::SocketAddr, sync::Arc};

use api::auth::logged_in;
use app::Base;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use data::app_state::AppState;
use http::{header, HeaderMap, HeaderValue, StatusCode};
use tokio::sync::watch;
use tower_cookies::{CookieManagerLayer, Cookies, Key};
use tower_http::services::ServeDir;
use tracing_subscriber::prelude::*;
use utils::ToServerError;

use crate::data::app_state::AppStateInner;

mod api;
mod app;
mod data;
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

    dotenvy::dotenv().unwrap();

    let pool = data::database_init().await.unwrap();

    data::init_tables(&pool).await.unwrap();

    let (sender, _) = watch::channel((-1, -1));
    
    let cookie_key_master = hex::decode(dotenvy::var("COOKIE_KEY").unwrap()).unwrap();

    let app_state = Arc::new(AppStateInner {
        pool,
        jws_key: dotenvy::var("JWS_SECRET").unwrap(),
        cookie_key: Key::from(&cookie_key_master),
        message_sent: sender,
    });

    // let m = hex::encode(app_state.cookie_key.master());
    // tracing::debug!("{}", m);

    let app = Router::new()
        .route("/", get(handler))
        .route("/:recipient", get(handler_chat))
        .route("/login", get(login))
        .route("/signup", get(signup))
        .nest("/api", api::api_routes())
        .route("/profile_pictures/:username", get(profile_pictures))
        .nest_service("/assets", ServeDir::new("assets/"))
        .fallback(not_found)
        .with_state(app_state)
        .layer(CookieManagerLayer::new());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<Result<Base, LandingPageTemplate>, StatusCode> {
    match logged_in(&state, &cookies).await.server_error()? {
        Some(user_id) => Ok(Ok(app::main(state, user_id, None).await?)),
        None => Ok(Err(LandingPageTemplate)),
    }
}

#[derive(Template)]
#[template(path = "landing_page.html")]
struct LandingPageTemplate;

async fn handler_chat(
    Path(recipient): Path<String>,
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<Result<impl IntoResponse, Redirect>, StatusCode> {
    tracing::debug!("handle chat");
    match logged_in(&state, &cookies).await.server_error()? {
        Some(user_id) => Ok(Ok(app::main(state, user_id, Some(recipient)).await?)),
        None => Ok(Err(Redirect::to("/"))),
    }
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    cookies: Cookies,
) -> Result<Result<LogInTemplate, Redirect>, StatusCode> {
    if headers
        .get("HX-Request")
        .is_some_and(|header| header == "true")
    {
        Ok(Ok(LogInTemplate::default()))
    } else {
        if logged_in(&state, &cookies).await.server_error()?.is_some() {
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
    State(state): State<AppState>,
    headers: HeaderMap,
    cookies: Cookies,
) -> Result<Result<SignUpTemplate, Redirect>, StatusCode> {
    if headers
        .get("HX-Request")
        .is_some_and(|header| header == "true")
    {
        return Ok(Ok(SignUpTemplate::default()));
    } else {
        if logged_in(&state, &cookies).await.server_error()?.is_some() {
            return Ok(Err(Redirect::to("/")));
        } else {
            return Ok(Ok(SignUpTemplate::default()));
        }
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
    _headers: HeaderMap,
    _cookies: Cookies,
) -> Result<Result<impl IntoResponse, Redirect>, StatusCode> {
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
        None => Err(StatusCode::NOT_FOUND),
    }
}
