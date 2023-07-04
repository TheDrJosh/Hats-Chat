use std::net::SocketAddr;

use axum::{Router, response::Html, routing::get};
use tower_http::{
    services::ServeDir,
};
use tracing_subscriber::prelude::*;

const HTML_BASE: &str = include_str!("../assets/base.html");

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "web_chat_app=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

    let app = Router::new().route("/", get(handler)).nest_service("/assets", ServeDir::new("assets/"));
    
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();


}

async fn handler() -> Html<&'static str> {
    Html(HTML_BASE)
}