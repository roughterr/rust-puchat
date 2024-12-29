use std::collections::HashMap;
use tungstenite::Message;
use crate::{connection_handler, dto};
use crate::user_context::user_context;

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
    let mut users_context: HashMap<String, user_context> = HashMap::new();

    // a lot should be added here
    for received in connection_command_receiver {
        match received {
            ConnectionCommand::AssignConnectionToUser {
                username,
                messages_sender,
            } => {
                println!("AssignConnectionToUser, username={}", &username);
                match users_context.get_mut(&username) {
                    Some(user_context) => {
                        let mut senders = &mut user_context.opened_sessions_senders;
                        if senders.len() as i32 >= connection_handler::MAXIMUM_SESSIONS_PER_USER {
                            let _ = messages_sender.send(Message::Text("Exceeded the limit of WebSocket connections".to_string()));
                            let _ = messages_sender.send(Message::Close(None));
                        } else {
                            senders.push(messages_sender);
                        }
                    }
                    None => {
                        let mut user_context = user_context::new();
                        // we assume that having 1 connection is always OK
                        user_context.opened_sessions_senders.push(messages_sender);
                        users_context.insert(username, user_context);
                    }
                }
            }
            ConnectionCommand::UnassignConnectionFromUser {
                username,
                messages_sender,
            } => {
                println!("UnassignConnectionFromUser, username={}", username);
                if let Some(user_context) = users_context.get_mut(&username) {
                    user_context.opened_sessions_senders.retain(|s| !s.same_channel(&messages_sender));
                }
            }
            ConnectionCommand::SendMessageToAnotherUser { username, content } => {
                println!(
                    "SendMessageToAnotherUser, username={}, message={}",
                    username, content
                );
                match users_context.get(&username) {
                    Some(user_context) => {
                        let message_obj = dto::prepare_message_for_from_server_to_client(username, content);
                        for sender in user_context.opened_sessions_senders.iter() {
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
        for (key, value) in &users_context {
            print!("User {} has {} opened connections. ", key, value.opened_sessions_senders.len());
        }
        println!();
    }
}