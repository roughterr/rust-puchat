use crate::dto;
use crate::user_context::{AddSessionResult, ApplicationScope, PrivateMessageServerMetadata};
use tungstenite::Message;
use crate::dto::{attach_subject_and_serialize, MessageToSomeone, MESSAGE_SUBJECT};

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
        sender_username: String,
        receiver_username: String,
        ///message content
        content: String,
    },
}

/// Receives events from the connections and does something.
pub async fn handle_connection_commands(
    connection_command_receiver: crossbeam_channel::Receiver<ConnectionCommand>,
) {
    let mut application_scope: ApplicationScope = ApplicationScope::new();

    // a lot should be added here
    for received in connection_command_receiver {
        match received {
            ConnectionCommand::AssignConnectionToUser {
                username,
                messages_sender,
            } => {
                println!("AssignConnectionToUser, username={}", &username);
                match application_scope.add_session_sender_if_not_exceeded(
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
                application_scope.remove_session_sender(&username, &messages_sender);
            }
            ConnectionCommand::SendMessageToAnotherUser { sender_username, receiver_username, content } => {
                let private_message_server_metadata: PrivateMessageServerMetadata =
                    application_scope.add_message_to_private_conversation(sender_username.clone(), receiver_username.clone(), content.clone());
                match application_scope.chat_users.get(&receiver_username) {
                    Some(user_context) => {
                        let message_obj =
                            dto::prepare_message_for_from_server_to_client(MessageToSomeone {
                                id: private_message_server_metadata.id,
                                content,
                                sender_username,
                                datetime: private_message_server_metadata.server_time.to_string(),
                            });
                        for sender in user_context.opened_sessions_senders.iter() {
                            let _ = sender.send(Message::Text(message_obj.clone()));
                        }
                    }
                    None => {
                        println!(
                            "cannot send a message {} to user {} right now because he is not connected",
                            content, receiver_username
                        );
                    }
                }
            }
        }
        // print hashmap
        for (key, value) in &application_scope.chat_users {
            print!(
                "User {} has {} opened connections. ",
                key,
                value.opened_sessions_senders.len()
            );
        }
        println!();
    }
}
