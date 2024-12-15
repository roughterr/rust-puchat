use serde::{Deserialize, Serialize};
use tungstenite::{connect, Message};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginCredentials {
    pub login: String,
    pub password: String,
    pub subject: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NewMessage {
    salt: String,
    content: String,
    subject: String,
    toWhom: String
}

fn main() {
    env_logger::init();

    let (mut socket, response) = connect("ws://127.0.0.1:8080").expect("Can't connect");

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");
    for (header, _value) in response.headers() {
        println!("* {header}");
    }

    let dan_authorization_message = LoginCredentials {
        subject: "authenticate".to_string(),
        login: "ian".to_string(),
        password: "ian".to_string(),
    };
    let authorization_in_json_format = serde_json::to_string(&dan_authorization_message);
    let _ = socket.send(Message::Text(authorization_in_json_format.unwrap()));
    loop {
        let msg = socket.read().expect("Error reading message");
        println!("Received: {msg}");
        if msg.to_string() == "authentication successful".to_string() {
            println!("authentication was successful");
            let new_message = NewMessage {
                salt: "now?".to_string(),
                content: "hi dan. how are you?".to_string(),
                subject: "new-message".to_string(),
                toWhom: "dan".to_string()
            };
            println!("new_message: {:?}", new_message);
            let new_message_in_json_format = serde_json::to_string(&new_message);
            println!("new_message_in_json_format.unwrap(): {:?}", new_message_in_json_format);
            let _ = socket.send(Message::Text(new_message_in_json_format.unwrap()));
        }
    }
}
