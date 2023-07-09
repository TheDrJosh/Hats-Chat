use sqlx::PgPool;

pub mod user;

pub async fn database_init() -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;
    Ok(pool)
}

pub async fn init_tables(pool: &PgPool) -> anyhow::Result<()> {
    user::User::init(&pool).await?;
    Ok(())
}
