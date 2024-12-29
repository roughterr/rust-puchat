use std::collections::HashMap;
use tungstenite::Message;

struct PuchatMessage {
    author: String,
    content: String,
    client_salt: String,
    server_time: i64,
}

type Conversation = Vec<PuchatMessage>;

pub struct user_context {
    /// the key is the user id of the Conversation partner
    pub owned_private_conversations: HashMap<String, Conversation>,
    pub not_owned_private_conversations: Vec<String>,
    pub opened_sessions_senders: Vec<crossbeam_channel::Sender<Message>>
}

impl user_context {
    /// Constructs a new `user_context`.
    pub fn new() -> Self {
        user_context {
            owned_private_conversations: HashMap::new(),
            not_owned_private_conversations: Vec::new(),
            opened_sessions_senders: Vec::new(),
        }
    }
}