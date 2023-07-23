use sqlx::PgPool;

pub async fn init_activations_table(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query!(
        "
    CREATE TABLE IF NOT EXISTS account_activation (
        id INT PRIMARY KEY,
        token TEXT NOT NULL,
        created TIMESTAMP NOT NULL,
        FOREIGN KEY (id) REFERENCES users (id)
    );"
    )
    .execute(pool)
    .await?;

    Ok(())
}
