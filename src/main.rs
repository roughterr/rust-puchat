mod dto;
mod user_service;

use dto::{LoginCredentials, MessageFromSomeone, Subject};
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use log::error;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use chrono::Utc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message, WebSocketStream};

use crossbeam_channel::unbounded;
use crate::dto::{MessageToSomeone, MESSAGE_SUBJECT};
use serde_json::{Value, json};

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init();

    // Get the address to bind to
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let addr: SocketAddr = addr.parse().expect("Invalid address");

    // Create the TCP listener
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");

    println!("Listening on: {}", addr);

    let (connection_command_sender, connection_command_receiver) = unbounded::<ConnectionCommand>();

    // listening to answers from handlers
    tokio::spawn(handle_connection_commands(connection_command_receiver));

    while let Ok((stream, _)) = listener.accept().await {
        // Spawn a new task for each connection
        tokio::spawn(handle_connection(stream, connection_command_sender.clone()));
    }
}

/// Receives events from the connections and does something.
async fn handle_connection_commands(
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
                println!("AssignConnectionToUser, username={}", username);
                user_to_connection_senders
                    .entry(username)
                    .and_modify(|vec| vec.push(messages_sender.clone()))
                    .or_insert_with(|| vec![messages_sender]);
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
                        // cook 2 json objects
                        let message_subject = Subject {
                            subject: MESSAGE_SUBJECT.to_string()
                        };
                        let message_to_someone = MessageToSomeone {
                            content,
                            sender: username,
                            salt:current_time_millis_as_string()
                        };
                        let message_json = serde_json::to_value(&message_to_someone).unwrap();
                        let message_subject_json = serde_json::to_value(&message_subject).unwrap();
                        // jsons are cooked
                        // Merge the JSON objects
                        let mut merged_json = message_json.as_object().unwrap().clone();
                        merged_json.extend(message_subject_json.as_object().unwrap().clone());
                        // Convert the merged map back to a JSON value
                        let final_json = Value::Object(merged_json);
                        let message_obj = Message::Text(final_json.to_string());
                        //TODO wrap in a
                        for sender in senders {
                            let _ = sender.send(message_obj.clone());
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
        println!(
            "user_to_connection_senders: {:?}",
            user_to_connection_senders
        );
    }
}

enum ConnectionCommand {
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

/*
* Sends a stream of messages to a WebSocket connection.
*/
async fn send_ws_messages_from_stream(
    mut ws_sender: SplitSink<WebSocketStream<TcpStream>, Message>,
    messages_receiver: crossbeam_channel::Receiver<Message>,
) {
    for message in messages_receiver {
        ws_sender.send(message).await.expect("TODO: panic message");
    }
}

async fn handle_connection(
    stream: TcpStream,
    connection_command_sender: crossbeam_channel::Sender<ConnectionCommand>,
) {
    // Accept the WebSocket connection
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("Error during the websocket handshake: {}", e);
            return;
        }
    };

    // Split the WebSocket stream into a sender and receiver
    let (ws_sender, mut ws_receiver) = ws_stream.split();

    // it lets this connection receive messages from other connections
    let (messages_sender, messages_receiver) = unbounded::<Message>();
    tokio::spawn(send_ws_messages_from_stream(ws_sender, messages_receiver));

    let mut current_username: String = String::new();
    let unsubscribe_closure =
        |username: String, messages_sender: crossbeam_channel::Sender<Message>| {
            if !username.is_empty() {
                // send a command to unsubscribe
                let _ =
                    connection_command_sender.send(ConnectionCommand::UnassignConnectionFromUser {
                        username,
                        messages_sender,
                    });
            }
        };

    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(content)) => {
                println!("Incoming message {:?}", &content);
                let subject: Subject =
                    serde_json::from_str(&content).expect("JSON was not well-formatted");
                println!("subject: {:#?}", &subject.subject);
                match subject.subject.as_str() {
                    "authenticate" => {
                        let login_credentials: LoginCredentials =
                            serde_json::from_str(&content).expect("JSON was not well-formatted");
                        let is_password_correct = user_service::are_credentials_correct(
                            &login_credentials.login,
                            &login_credentials.password,
                        );
                        println!("is_password_correct = {}", is_password_correct);
                        if is_password_correct {
                            let _ = messages_sender
                                .send(Message::Text("authentication successful".to_owned()));
                            current_username = login_credentials.login;
                            let _ = connection_command_sender.send(
                                ConnectionCommand::AssignConnectionToUser {
                                    username: current_username.clone(),
                                    messages_sender: messages_sender.clone(),
                                },
                            );
                        } else {
                            let _ = messages_sender.send(Message::Text(
                                "provide correct login and password for authentication".to_owned(),
                            ));
                        }
                    }
                    "new-message" => {
                        println!("sending message to another user");
                        if current_username.is_empty() {
                            let _ = messages_sender.send(Message::Text(
                                "you should authorize before sending messages to other users"
                                    .to_owned(),
                            ));
                        } else {
                            // parse the message
                            let new_message: MessageFromSomeone = serde_json::from_str(&content)
                                .expect("JSON was not well-formatted");
                            let _ = connection_command_sender.send(
                                ConnectionCommand::SendMessageToAnotherUser {
                                    username: new_message.receiver,
                                    content: new_message.content,
                                },
                            );
                        }
                    }
                    _ => {
                        let _ = messages_sender.send(Message::Text("unknown subject".to_owned()));
                        // Close the WebSocket connection gracefully
                        let _ = messages_sender.send(Message::Close(None));
                        println!("Close frame sent because the subject was unknown");
                        //sender.shutdown().await.unwrap();
                    }
                }
            }
            Ok(Message::Close(_)) => {
                println!("The client wants to gracefully close the session");
                unsubscribe_closure(current_username, messages_sender);
                break;
            }
            Ok(_) => print!("OK_"),
            Err(e) => {
                error!("Error: {}", e);
                unsubscribe_closure(current_username, messages_sender);
                break;
            }
        }
        println!("end of function handle_connection");
    }
}

fn current_time_millis_as_string() -> String {
    // Get the current time in UTC
    let now = Utc::now();
    // Convert to milliseconds since the UNIX epoch
    let millis = now.timestamp_millis();
    // Convert the milliseconds to a string
    millis.to_string()
}