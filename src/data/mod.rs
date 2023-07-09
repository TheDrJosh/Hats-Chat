pub mod app_state;

use sqlx::PgPool;

pub async fn database_init() -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;
    Ok(pool)
}

pub async fn init_tables(pool: &PgPool) -> anyhow::Result<()> {
    init_user_tables(&pool).await?;
    Ok(())
}

pub async fn init_user_tables(pool: &PgPool) -> anyhow::Result<()> {
    // sqlx::query!("DROP TABLE IF EXISTS aut_tokens, users;").execute(pool).await?;
    sqlx::query!(
        "
    CREATE TABLE IF NOT EXISTS users (
        id SERIAL PRIMARY KEY,
        username TEXT UNIQUE NOT NULL,
        display_name TEXT,
        email TEXT UNIQUE NOT NULL,
        password_hash TEXT NOT NULL
    );"
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        "
    CREATE TABLE IF NOT EXISTS auth_tokens (
        token TEXT PRIMARY KEY,
        user_id INT NOT NULL,
        FOREIGN KEY (user_id) REFERENCES users (id)
    );"
    )
    .execute(pool)
    .await?;
    Ok(())
}
