use sqlx::PgPool;


pub struct User {
    pub id: Option<i32>,
    pub username: String,
    pub display_name: Option<String>,
    pub email: String,
    pub password_hash: String,
    pub active_tokens: Vec<String>,
}

impl User {
    pub async fn init(pool: &PgPool) -> anyhow::Result<()> {
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
            user_id INT NOT NULL UNIQUE,
            FOREIGN KEY (user_id) REFERENCES users (id)
        );"
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get(id: i32, pool: &PgPool) -> anyhow::Result<User> {
        let user = sqlx::query!(
            "
            SELECT * FROM users WHERE id = $1;
        ",
            id
        )
        .fetch_one(pool)
        .await?;

        let tokens = sqlx::query!(
            "
            SELECT * FROM auth_tokens WHERE user_id = $1;
        ",
            user.id
        )
        .fetch_all(pool)
        .await?
        .iter()
        .map(|record| record.token.clone()).collect();

        Ok(User {
            id: Some(user.id),
            username: user.username,
            display_name: user.display_name,
            email: user.email,
            password_hash: user.password_hash,
            active_tokens: tokens,
        })
    }
}
