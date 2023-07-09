use std::sync::Arc;

use sqlx::PgPool;
use tower_cookies::Key;

pub struct AppStateInner {
    pub pool: PgPool,
    pub jws_key: String,
    pub cookie_key: Key,
}

pub type AppState = Arc<AppStateInner>;
