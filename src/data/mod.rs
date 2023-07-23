pub mod app_state;
mod chat;
mod users;
mod activated;

use sqlx::PgPool;

use self::{chat::init_chat_table, users::init_user_tables, activated::init_activations_table};

pub async fn database_init() -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;
    Ok(pool)
}

pub async fn init_tables(pool: &PgPool) -> anyhow::Result<()> {
    init_user_tables(pool).await?;
    init_chat_table(pool).await?;
    init_activations_table(pool).await?;
    Ok(())
}
