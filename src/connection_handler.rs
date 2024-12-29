use std::collections::HashMap;
use tungstenite::Message;
use crate::{connection_handler, dto};

/// Define the maximum allowed number of WebSocket connections per user.
pub const MAXIMUM_SESSIONS_PER_USER: i32 = 2;

pub enum ConnectionCommand {
    AssignConnectionToUser {
        username: String,
        messages_sender: crossbeam_channel::Sender<Message>,
    },
    UnassignConnectionFromUser {
        username: String,
        messages_sender: crossbeam_channel::Sender<Message>,
    },
    SendMessageToAnotherUser {
        username: String,
        ///message content
        content: String,
    },
}

/// Receives events from the connections and does something.
pub async fn handle_connection_commands(
    connection_command_receiver: crossbeam_channel::Receiver<ConnectionCommand>,
) {
    let mut user_to_connection_senders: HashMap<String, Vec<crossbeam_channel::Sender<Message>>> =
        HashMap::new();

    // a lot should be added here
    for received in connection_command_receiver {
        match received {
            ConnectionCommand::AssignConnectionToUser {
                username,
                messages_sender,
            } => {
                println!("AssignConnectionToUser, username={}", &username);
                match user_to_connection_senders.get_mut(&username) {
                    Some(senders) => {
                        if senders.len() as i32 >= connection_handler::MAXIMUM_SESSIONS_PER_USER {
                            let _ = messages_sender.send(Message::Text("Exceeded the limit of WebSocket connections".to_string()));
                            let _ = messages_sender.send(Message::Close(None));
                        } else {
                            senders.push(messages_sender);
                        }
                    }
                    None => {
                        // we assume that having 1 connection is always OK
                        user_to_connection_senders.insert(username, vec![messages_sender]);
                    }
                }
            }
            ConnectionCommand::UnassignConnectionFromUser {
                username,
                messages_sender,
            } => {
                println!("UnassignConnectionFromUser, username={}", username);
                if let Some(vec) = user_to_connection_senders.get_mut(&username) {
                    vec.retain(|s| !s.same_channel(&messages_sender));
                    if vec.is_empty() {
                        user_to_connection_senders.remove(&username);
                    }
                }
            }
            ConnectionCommand::SendMessageToAnotherUser { username, content } => {
                println!(
                    "SendMessageToAnotherUser, username={}, message={}",
                    username, content
                );
                match user_to_connection_senders.get(&username) {
                    Some(senders) => {
                        let message_obj = dto::prepare_message_for_from_server_to_client(username, content);
                        for sender in senders {
                            let _ = sender.send(Message::Text(message_obj.clone()));
                        }
                    }
                    None => {
                        println!(
                            "cannot send a message {} to user {} because he is not connected",
                            content, username
                        );
                    }
                }
            }
        }
        // print hashmap
        for (key, value) in &user_to_connection_senders {
            print!("User {} has {} opened connections. ", key, value.len());
        }
        println!();
    }
}