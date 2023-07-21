use std::sync::Arc;

use lettre::SmtpTransport;
use sqlx::PgPool;
use tokio::sync::watch;
use tower_cookies::Key;

pub struct AppStateInner {
    pub pool: PgPool,
    pub jws_key: String,
    pub cookie_key: Key,
    pub message_sent: watch::Sender<(i32, i32)>,
    pub mailer: SmtpTransport,
}

pub type AppState = Arc<AppStateInner>;
