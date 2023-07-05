use axum::{Router, routing::post, Form};

pub fn auth_routes() -> Router {
    Router::new()
        .route("/user/create",  post(create))
        .route("/user/login", post(login))
        .route("/user/logout", post(logout))
}

async fn create() -> () {
    todo!()
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login(Form(form): Form<LoginForm>) -> () {
    println!("{:?}", form);
    todo!()
}

async fn logout() -> () {
    todo!()
}