use serde::Deserialize;

// this file will contain data transfer objects

#[derive(Debug, Deserialize)]
pub struct LoginCredentials {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct NewMessage {
    pub salt: String,
    pub content: String,
    pub toWhom: String,
}

#[derive(Debug, Deserialize)]
pub struct Subject {
    pub subject: String,
}
