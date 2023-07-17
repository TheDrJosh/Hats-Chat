use sqlx::PgPool;

pub async fn init_user_tables(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query!(
        "
    CREATE TABLE IF NOT EXISTS users (
        id SERIAL PRIMARY KEY,
        username TEXT UNIQUE NOT NULL,
        display_name TEXT,
        profile_picture BYTEA,
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

    // can remove after fix of persistant cookie key
    // sqlx::query!("DELETE FROM auth_tokens")
    //     .execute(pool)
    //     .await?;

    Ok(())
}
