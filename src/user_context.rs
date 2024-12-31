use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use tungstenite::Message;
use crate::user_context::AddSessionResult::{Success, TooManySessions};

/// Represents a private chat message in the server internal memory.
struct PrivateMessage {
    /// message id. Message 0 is the chat itself!
    id: u32,
    /// whether the conversation owner is the author of the message.
    is_owner_author: bool,
    /// the content of the message.
    content: String,
    /// the time (in milliseconds) when the server received the message.
    server_time: DateTime<Utc>,
}

struct PrivateConversation {
    /// if the user is the owner of the conversation
    owner: bool,
    /// It will be empty if the user is not the owner of the conversation.
    messages: Vec<PrivateMessage>,
    /// how many times this conversation has been shown to the user
    show_count: u32,
    /// HashMap where the key is the number of times when the user retrieved the conversation.
    /// And the value is the number of messages that were sent as follower messages to this show count.
    /// It is done to prevent a situation when the user sends a few consecutive messages but the fist
    /// message is lost. We do not want the user to see the second or any other consecutive messages
    /// until the previous are delivered.
    show_count_to_following_messages_count: HashMap<u32, u16>,
}

/// The data of one user.
pub struct ConversationPartner {
    /// the key is the user id of the Conversation partner.
    pub private_conversations: HashMap<String, PrivateConversation>,
    // the currently opened sessions of the users.
    pub opened_sessions_senders: Vec<crossbeam_channel::Sender<Message>>,
}

impl ConversationPartner {
    /// Constructs a new `ConversationPartner`.
    pub fn new() -> Self {
        ConversationPartner {
            private_conversations: HashMap::new(),
            opened_sessions_senders: Vec::new(),
        }
    }
}

/// The data about all users.
pub struct ConversationPartners {
    pub conversation_partners: HashMap<String, ConversationPartner>,
}

pub enum AddSessionResult {
    Success,
    TooManySessions { messages_sender: crossbeam_channel::Sender<Message> }
}

impl ConversationPartners {
    pub fn new() -> Self {
        ConversationPartners {
            conversation_partners: HashMap::new(),
        }
    }

    pub fn add_session_sender_if_not_exceeded(&mut self, username: &String,
                                              messages_sender: crossbeam_channel::Sender<Message>,
                                              maximum_sessions_allowed: i32)
                                              -> AddSessionResult {
        match self.conversation_partners.get_mut(username) {
            None => {
                // create a new conversation partner
                let mut conversation_partner: ConversationPartner = ConversationPartner::new();
                conversation_partner.opened_sessions_senders.push(messages_sender);
                // register the new conversation partner
                self.conversation_partners.insert(username.clone(), conversation_partner);
                Success
            }
            Some(conversation_partner) => {
                if conversation_partner.opened_sessions_senders.len() as i32 >= maximum_sessions_allowed {
                    TooManySessions {messages_sender}
                } else {
                    conversation_partner.opened_sessions_senders.push(messages_sender);
                    Success
                }
            }
        }
    }

    pub fn remove_session_sender(&mut self, username: &String, messages_sender: &crossbeam_channel::Sender<Message>,) {
        match self.conversation_partners.get_mut(username) {
            None => {
            }
            Some(conversation_partner) => {
                conversation_partner.opened_sessions_senders.retain(|s| !s.same_channel(&messages_sender));
            }
        }
    }
}

// partner1 is the reader!
// pub fn read_last_messages_of_private_conversation(&self, partner1: &String, partner2: &String, n: u16) -> Option<PrivateConversation> {
//     match self.conversation_partners.get(partner1) {
//         Some(conversation_partner) => {
//             conversation_partner.
//         }
//         None => {}
//     }
// }
// }