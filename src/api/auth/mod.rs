use std::collections::HashMap;

use axum::{Router, routing::post, Form, response::{Html, Redirect}, body::{HttpBody, Body}};
use http::StatusCode;
use serde::Deserialize;

pub fn auth_routes() -> Router {
    Router::new()
        .route("/user/create",  post(create))
        .route("/user/login", post(login))
        .route("/user/logout", post(logout))
}

#[derive(Debug, Deserialize)]
struct CreateUserForm {
    username: String,
    email: String,
    password: String,
    confirm_password: String,
}

async fn create(Form(form): Form<CreateUserForm>) -> Result<Html<String>, StatusCode> {
    println!("{:?}", form);
    Ok(Html(String::new()))
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login(Form(form): Form<LoginForm>) -> Result<Html<String>, StatusCode> {

    println!("{:?}", form);

    Ok(Html(String::new()))
}

async fn logout() -> () {
}