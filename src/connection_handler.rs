use crate::dto;
use crate::user_context::{AddSessionResult, ApplicationScope};
use tungstenite::Message;

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
    let mut conversation_partners: ApplicationScope = ApplicationScope::new();

    // a lot should be added here
    for received in connection_command_receiver {
        match received {
            ConnectionCommand::AssignConnectionToUser {
                username,
                messages_sender,
            } => {
                println!("AssignConnectionToUser, username={}", &username);
                match conversation_partners.add_session_sender_if_not_exceeded(
                    &username,
                    messages_sender,
                    MAXIMUM_SESSIONS_PER_USER,
                ) {
                    AddSessionResult::Success => {}
                    AddSessionResult::TooManySessions { messages_sender } => {
                        let _ = messages_sender.send(Message::Text("Exceeded the limit of WebSocket connections".to_string()));
                        let _ = messages_sender.send(Message::Close(None));
                    }
                }
            }
            ConnectionCommand::UnassignConnectionFromUser {
                username,
                messages_sender,
            } => {
                println!("UnassignConnectionFromUser, username={}", username);
                conversation_partners.remove_session_sender(&username, &messages_sender);
            }
            ConnectionCommand::SendMessageToAnotherUser { username, content } => {
                println!(
                    "SendMessageToAnotherUser, username={}, message={}",
                    username, content
                );
                match conversation_partners.chat_users.get(&username) {
                    Some(user_context) => {
                        let message_obj =
                            dto::prepare_message_for_from_server_to_client(username, content);
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
        for (key, value) in &conversation_partners.chat_users {
            print!(
                "User {} has {} opened connections. ",
                key,
                value.opened_sessions_senders.len()
            );
        }
        println!();
    }
}
