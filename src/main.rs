use std::{net::SocketAddr, sync::Arc};

use api::auth::logged_in;
use axum::{
    extract::State,
    response::{Html, Redirect},
    routing::get,
    Router,
};
use data::app_state::AppState;
use http::{HeaderMap, StatusCode};
use tokio::fs;
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

    let app_state = Arc::new(AppStateInner {
        pool,
        jws_key: dotenvy::var("JWS_SECRET").unwrap(),
        cookie_key: Key::generate(),
    });

    let app = Router::new()
        .route("/", get(handler))
        .route("/login", get(login))
        .route("/signup", get(signup))
        .nest("/api", api::api_routes())
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
) -> Result<Html<String>, StatusCode> {
    match logged_in(&state, &cookies).await.server_error()? {
        Some(user_id) => app::main(state, user_id).await,
        None => Ok(Html(fs::read_to_string("pages/landing_page.html").await.unwrap())),
    }
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    cookies: Cookies,
) -> Result<Result<Html<String>, Redirect>, StatusCode> {
    if headers
        .get("HX-Request")
        .is_some_and(|header| header == "true")
    {
        Ok(Ok(Html(login_partial(""))))
    } else {
        if logged_in(&state, &cookies).await.server_error()?.is_some() {
            return Ok(Err(Redirect::to("/")));
        }
        Ok(Ok(Html(format!(
            include_str!("../pages/auth_template.html"),
            login_partial("")
        ))))
    }
}

pub fn login_partial(under_username: &str) -> String {
    format!(include_str!("../pages/partial/log-in.html"), under_username)
}

async fn signup(
    State(state): State<AppState>,
    headers: HeaderMap,
    cookies: Cookies,
) -> Result<Result<Html<String>, Redirect>, StatusCode> {
    if headers
        .get("HX-Request")
        .is_some_and(|header| header == "true")
    {
        Ok(Ok(Html(signup_partial("", "", ""))))
    } else {
        if logged_in(&state, &cookies).await.server_error()?.is_some() {
            return Ok(Err(Redirect::to("/")));
        }
        Ok(Ok(Html(format!(
            include_str!("../pages/auth_template.html"),
            signup_partial("", "", "")
        ))))
    }
}

fn signup_partial(under_username: &str, under_email: &str, under_password_confirm: &str) -> String {
    format!(
        include_str!("../pages/partial/sign-up.html"),
        under_username, under_email, under_password_confirm
    )
}

async fn not_found(State(_state): State<AppState>) -> Html<String> {
    Html(fs::read_to_string("pages/not_found.html").await.unwrap())
}
