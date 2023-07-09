use std::net::SocketAddr;

use axum::{response::Html, routing::get, Router};
use http::HeaderMap;
use tokio::fs;
use tower_http::services::ServeDir;
use tracing_subscriber::prelude::*;

mod api;
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

    let app = Router::new()
        .route("/", get(handler))
        .route("/login", get(login))
        .route("/signup", get(signup))
        .nest("/api", api::api_routes(pool))
        .nest_service("/assets", ServeDir::new("assets/"))
        .fallback(not_found);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<String> {
    Html(fs::read_to_string("pages/landing_page.html").await.unwrap())
}

async fn login(headers: HeaderMap) -> Html<String> {
    if headers.get("HX-Request").is_some_and(|header| header == "true") {
        Html(login_partial(""))
    } else {
        Html(format!(
            include_str!("../pages/auth_template.html"),
            login_partial("")
        ))
    }
}

pub fn login_partial(under_username: &str) -> String {
    format!(include_str!("../pages/partial/log-in.html"), under_username)
}

async fn signup(headers: HeaderMap) -> Html<String> {
    if headers.get("HX-Request").is_some_and(|header| header == "true") {
        Html(signup_partial("", "", ""))
    } else {
        Html(format!(
            include_str!("../pages/auth_template.html"),
            signup_partial("", "", "")
        ))
    }
}

fn signup_partial(under_username: &str, under_email: &str, under_password_confirm: &str) -> String {
    format!(
        include_str!("../pages/partial/sign-up.html"),
        under_username, under_email, under_password_confirm
    )
}

async fn not_found() -> Html<String> {
    Html(fs::read_to_string("pages/not_found.html").await.unwrap())
}
