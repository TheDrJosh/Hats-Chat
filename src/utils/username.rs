

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
    pub fn username(&self) -> String {
        self.username.clone()
    }
    pub fn display_name(&self) -> String {
        self.display_name.clone().unwrap_or(self.username.clone())
    }
}
