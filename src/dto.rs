use erased_serde as erased;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Deref;

// this file will contain data transfer objects

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginCredentials {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageFromSomeone {
    pub message_sequence_id: u32,
    pub message_sequence_index: u16,
    pub content: String,
    pub receiver: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageToSomeone {
    pub id: u32,
    pub content: String,
    pub sender_username: String,
    pub datetime: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Subject {
    pub subject: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NewPrivateMessageSequenceRequest {
    pub receiver_username: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NewPrivateMessageSequenceResponse {
    pub receiver_username: String,
    pub sequence_id: u32,
}

pub const MESSAGE_SUBJECT: &'static str = "message";
pub const AUTHENTICATE_SUBJECT: &'static str = "authenticate";
pub const NEW_MESSAGE_SUBJECT: &'static str = "new-message";
pub const NEW_PRIVATE_MESSAGE_SEQUENCE_SUBJECT: &'static str = "new-private-message-sequence";

/// Accepts 2 objects: a struct with the main data for the request and a string with the message subject.
pub fn attach_subject_and_serialize(json_main_data: Box<dyn erased::Serialize>, subject: String) -> String {
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

/// Generates a string that the server sends to a client to send him a message from another user
pub fn prepare_message_for_from_server_to_client(message_to_someone: MessageToSomeone) -> String {
    attach_subject_and_serialize(
        Box::new(message_to_someone),
        MESSAGE_SUBJECT.to_string(),
    )
}
