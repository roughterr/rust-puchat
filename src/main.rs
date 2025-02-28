mod connection_handler;
mod dto;
mod private_conversation_partners;
mod user_context;
mod user_service;
mod util;

use dto::{LoginCredentials, MessageFromSomeone, MessageToSomeone, Subject, MESSAGE_SUBJECT};
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use log::error;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message, WebSocketStream};

use crate::connection_handler::{handle_connection_commands, ConnectionCommand};
use crate::dto::NewPrivateMessageSequenceRequest;
use crossbeam_channel::unbounded;
use serde_json::{json, Value};

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

/// Sends a stream of messages to a WebSocket connection.
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
                    dto::AUTHENTICATE_SUBJECT => {
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
                    dto::NEW_MESSAGE_SUBJECT => {
                        println!("NEW_MESSAGE_SUBJECT request");
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
                                    sender_username: current_username.clone(),
                                    receiver_username: new_message.receiver,
                                    content: new_message.content,
                                    message_sequence_id: new_message.message_sequence_id,
                                    message_sequence_index: new_message.message_sequence_index,
                                },
                            );
                        }
                    }
                    dto::NEW_PRIVATE_MESSAGE_SEQUENCE_SUBJECT => {
                        println!("NEW_PRIVATE_MESSAGE_SEQUENCE_SUBJECT request");
                        if current_username.is_empty() {
                            let _ = messages_sender.send(Message::Text(
                                "you should authorize before making this type of request"
                                    .to_owned(),
                            ));
                        } else {
                            match serde_json::from_str::<NewPrivateMessageSequenceRequest>(&content)
                            {
                                Ok(request) => {
                                    let _ = connection_command_sender.send(
                                        ConnectionCommand::InitiateNewPrivateMessageSequence {
                                            sender_username: current_username.clone(),
                                            receiver_username: request.receiver_username,
                                            messages_sender: messages_sender.clone(),
                                        },
                                    );
                                }
                                Err(e) => {
                                    eprintln!("Failed to parse JSON: {}", e);
                                    //TODO Handle the error appropriately, e.g., return a message with an error code
                                }
                            };
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
