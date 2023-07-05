use std::net::SocketAddr;

use axum::{response::Html, routing::get, Router};
use tokio::fs;
use tower_http::services::ServeDir;
use tracing_subscriber::prelude::*;

mod api;
mod data;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "web_chat_app=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .route("/", get(handler))
        .route("/login", get(login))
        .route("/partial/signup", get(partial_signup))
        .nest("/api", api::api_routes())
        .nest_service("/assets", ServeDir::new("assets/")).fallback(not_found);

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

async fn login() -> Html<String> {
    Html(fs::read_to_string("pages/log-in.html").await.unwrap())
}

async fn not_found() -> Html<String> {
    Html(fs::read_to_string("pages/not_found.html").await.unwrap())
}

async fn partial_signup() -> Html<String> {
    Html(fs::read_to_string("pages/partial/sign-up.html").await.unwrap())
}