use chrono::Utc;
use erased_serde as erased;
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use std::ops::Deref;
// this file will contain data transfer objects

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginCredentials {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageFromSomeone {
    pub salt: String,
    pub content: String,
    pub receiver: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageToSomeone {
    pub salt: String,
    pub content: String,
    pub sender: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Subject {
    pub subject: String,
}

pub const MESSAGE_SUBJECT: &'static str = "message";
pub const AUTHENTICATE_SUBJECT: &'static str = "authenticate";
pub const NEW_MESSAGE_SUBJECT: &'static str = "new-message";

/// Accepts 2 objects: a struct with the main data for the request and a string with the message subject.
pub fn prepare_json(json_main_data: Box<dyn erased::Serialize>, subject: String) -> String {
    let message_subject = Subject { subject };
    let message_json = serde_json::to_value(&json_main_data.deref()).unwrap();
    let message_subject_json = serde_json::to_value(&message_subject).unwrap();
    // jsons are cooked
    // Merge the JSON objects
    let mut merged_json = message_json.as_object().unwrap().clone();
    merged_json.extend(message_subject_json.as_object().unwrap().clone());
    // Convert the merged map back to a JSON value
    let final_json = Value::Object(merged_json);
    final_json.to_string()
}

pub fn current_time_millis_as_string() -> String {
    // Get the current time in UTC
    let now = Utc::now();
    // Convert to milliseconds since the UNIX epoch
    let millis = now.timestamp_millis();
    // Convert the milliseconds to a string
    millis.to_string()
}

/// Generates a string that the server sends to a client to send him a message from another user
pub fn prepare_message_for_from_server_to_client(sender: String, content: String) -> String {
    prepare_json(
        Box::new(MessageToSomeone {
            content,
            sender,
            salt: current_time_millis_as_string(),
        }),
        MESSAGE_SUBJECT.to_string(),
    )
}
