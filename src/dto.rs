use serde::{Deserialize, Serialize};

// this file will contain data transfer objects

#[derive(Debug, Deserialize)]
pub struct LoginCredentials {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct MessageFromSomeone {
    pub salt: String,
    pub content: String,
    pub receiver: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Subject {
    pub subject: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageToSomeone {
    pub salt: String,
    pub content: String,
    pub sender: String,
}

pub const MESSAGE_SUBJECT: &'static str = "message";