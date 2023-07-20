use sqlx::PgPool;




#[derive(Debug, Clone)]
pub struct Username {
    username: String,
    display_name: Option<String>
}

impl Username {
    pub fn new(username: String, display_name: Option<String>) -> Self {
        Self {
            username,
            display_name,
        }
    }

    pub async fn new_from_id(user_id: i32, pool: &PgPool) -> anyhow::Result<Self> {
        let rec = sqlx::query!("SELECT username, display_name FROM users WHERE id = $1", user_id).fetch_one(pool).await?;

        Ok(Self::new(rec.username, rec.display_name))
    }

    pub fn username(&self) -> String {
        self.username.clone()
    }

    pub fn display_name(&self) -> String {
        self.display_name.clone().unwrap_or(self.username.clone())
    }
}
