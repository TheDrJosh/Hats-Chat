use sqlx::PgPool;

pub async fn init_chat_table(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query!(
        "
    CREATE TABLE IF NOT EXISTS chat_messages (
        id SERIAL PRIMARY KEY,
        sender_id INT NOT NULL,
        recipient_id INT NOT NULL,
        msg TEXT NOT NULL,
        sent_at TIMESTAMP NOT NULL,
        FOREIGN KEY (sender_id) REFERENCES users (id),
        FOREIGN KEY (recipient_id) REFERENCES users (id)
    );"
    )
    .execute(pool)
    .await?;

    Ok(())
}
