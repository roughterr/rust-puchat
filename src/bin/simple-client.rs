use rust_pr::dto::LoginCredentials;
use rust_pr::{dto, util};

use crossbeam_channel::{unbounded, Sender};
use futures::stream::SplitStream;
use futures::{SinkExt, StreamExt};
use std::io::{self, BufRead, Write};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::Message;

#[tokio::main]
async fn main() {
    // it you won't do the following 2 lines it you will see the error
    // "there is no reactor running, must be called from the context of a Tokio 1.x runtime"
    // see https://users.rust-lang.org/t/no-reactor-running-when-calling-runtime-spawn/81256/7
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _enter = rt.enter();

    println!("Connecting to WebSocket...");
    // let (mut socket, response) = connect("ws://127.0.0.1:8080").expect("Can't connect");
    // Connect to the WebSocket server
    let url = "ws://127.0.0.1:8080";
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    // Split the WebSocket into sender and receiver
    let (mut ws_sender, ws_receiver) = ws_stream.split();

    // receivers and senders of commands
    let (state_change_sender, state_change_receiver) = unbounded::<StateChange>();
    // listening to console input and websocket messages can be done in parallel
    rt.spawn(read_lines(state_change_sender.clone()));
    rt.spawn(read_ws_messages(state_change_sender.clone(), ws_receiver));

    let mut app_state = AppState::WaitingForUsername;
    print!("{}", "Please enter your login: ");
    io::stdout().flush().unwrap();

    for state_change in state_change_receiver {
        match state_change {
            StateChange::NewReadlineMessage { message } => {
                match app_state {
                    AppState::WaitingForUsername => {
                        print!("{}", "Please enter your password: ");
                        io::stdout().flush().unwrap();
                        app_state = AppState::WaitingForPassword { username: message };
                    }
                    AppState::WaitingForPassword { username } => {
                        let login_credentials = LoginCredentials {
                            login: username.to_string(),
                            password: message
                        };
                        let authorization_message = dto::attach_subject_and_serialize(Box::new(login_credentials), dto::AUTHENTICATE_SUBJECT.to_string());
                        let _ = ws_sender
                            .send(Message::Text(authorization_message))
                            .await
                            .unwrap();
                        println!("Authorization request sent.");
                        app_state = AppState::WaitingForServerAuthorizationResponse;
                    }
                    AppState::WaitingForServerAuthorizationResponse => {
                        println!("Please wait until the server sends us a reponse to our authorization request");
                    }
                    AppState::WaitingForReceiverName => {
                        // message should contain the username of the receiver
                        if is_valid_username(&message) {
                            print!("{}", "Please type the text that you want to send: ");
                            std::io::stdout().flush().unwrap();
                            app_state = AppState::WaitingForText {
                                receiver_name: message,
                            };
                        } else {
                            print!(
                                "{}",
                                "Please enter a valid username (only alphanumeric characters): "
                            );
                            std::io::stdout().flush().unwrap();
                        }
                    }
                    AppState::WaitingForText { receiver_name } => {
                        let new_message = dto::MessageFromSomeone {
                            salt: util::current_time_millis_as_string(),
                            content: message,
                            receiver: receiver_name.to_string(),
                        };
                        let new_message_str = dto::attach_subject_and_serialize(Box::new(new_message), dto::NEW_MESSAGE_SUBJECT.to_string());
                        let _ = ws_sender
                            .send(Message::Text(new_message_str))
                            .await
                            .unwrap();
                        println!("The message has been sent.");
                        print!(
                            "{}",
                            "Please enter the login of a user to whom you want to send a message: "
                        );
                        std::io::stdout().flush().unwrap();
                        app_state = AppState::WaitingForReceiverName;
                    }
                }
            }
            StateChange::NewWebSocketMessage { message } => {
                match app_state {
                    AppState::WaitingForServerAuthorizationResponse => {
                        if message == "authentication successful" {
                            print!("{}", "Please enter the login of a user to whom you want to send a message: ");
                            std::io::stdout().flush().unwrap();
                            app_state = AppState::WaitingForReceiverName;
                        } else {
                            println!("We were waiting for \"authentication successful\" but received something else: {}", &message);
                            print!("{}", "Please enter the username again: ");
                            std::io::stdout().flush().unwrap();
                            app_state = AppState::WaitingForUsername;
                        }
                    }
                    _ => {
                        println!(
                            "we have just received this message from the server: {}",
                            &message
                        )
                    }
                }
            }
        }
    }
}

fn is_valid_username(username: &str) -> bool {
    // Check if the string is not empty and contains only alphanumeric characters
    !username.is_empty() && username.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

enum AppState {
    WaitingForUsername,
    WaitingForPassword { username: String },
    WaitingForServerAuthorizationResponse,
    WaitingForReceiverName,
    WaitingForText { receiver_name: String },
}

enum StateChange {
    NewReadlineMessage { message: String },
    NewWebSocketMessage { message: String },
}

async fn read_ws_messages(
    state_change_sender: Sender<StateChange>,
    mut ws_receiver: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) {
    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(content)) => {
                let _ =
                    state_change_sender.send(StateChange::NewWebSocketMessage { message: content });
            }
            Ok(Message::Close(_)) => {
                println!("The server wants to gracefully close the session");
                break;
            }
            Ok(_) => print!("OK_"),
            Err(e) => {
                println!("WebSocket error: {}", e);
                break;
            }
        }
    }
}

async fn read_lines(state_change_sender: Sender<StateChange>) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let _ = state_change_sender.send(StateChange::NewReadlineMessage {
            message: line.unwrap(),
        });
    }
}
