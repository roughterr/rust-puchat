use crate::user_context::AddSessionResult::{Success, TooManySessions};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::hash::Hash;
use tungstenite::Message;

/// Metadata that the server add to a private message after the server receives the message.
pub struct PrivateMessageServerMetadata {
    /// message id
    id: u32,
    /// the datetime when the message was registered on the server.
    server_time: DateTime<Utc>,
}

/// Represents a private chat message in the server internal memory.
struct PrivateMessage {
    /// whether the conversation owner is the author of the message.
    is_owner_author: bool,
    /// the content of the message.
    content: String,
    /// the time (in milliseconds) when the server received the message.
    server_time: DateTime<Utc>,
    /// True for deleted messages.
    is_deleted: bool,
}

impl PrivateMessage {
    pub fn new(is_owner_author: bool, content: String) -> Self {
        PrivateMessage {
            is_owner_author,
            content,
            server_time: Utc::now(),
            is_deleted: false,
        }
    }
}

struct PrivateMessages {
    /// Defines from which index the array "messages" start. The default value must be 0.
    id_offset: u32,
    /// A list of messages.
    messages: Vec<PrivateMessage>,
}

struct PrivateConversation {
    /// It will be empty if the user is not the owner of the conversation.
    messages: Option<PrivateMessages>,
    /// how many times this conversation has been shown to the user
    show_count: u32,
    /// HashMap where the key is the number of times when the user retrieved the conversation.
    /// And the value is the number of messages that were sent as follower messages to this show count.
    /// It is done to prevent a situation when the user sends a few consecutive messages but the fist
    /// message is lost. We do not want the user to see the second or any other consecutive messages
    /// until the previous are delivered.
    show_count_to_following_messages_count: HashMap<u32, u16>,
}

impl PrivateConversation {
    pub fn new_owned_private_conversation(first_message_content: String) -> Self {
        PrivateConversation {
            messages: Some(PrivateMessages {
                id_offset: 0,
                messages: vec![PrivateMessage::new(true, first_message_content)],
            }),
            show_count: 0,
            show_count_to_following_messages_count: HashMap::new(),
        }
    }

    pub fn new_non_owned_private_conversation() -> Self {
        PrivateConversation {
            messages: None,
            show_count: 0,
            show_count_to_following_messages_count: HashMap::new(),
        }
    }
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
pub struct ApplicationScope {
    pub conversation_partners: HashMap<String, ConversationPartner>,
}

pub enum AddSessionResult {
    Success,
    TooManySessions {
        messages_sender: crossbeam_channel::Sender<Message>,
    },
}

impl ApplicationScope {
    pub fn new() -> Self {
        ApplicationScope {
            conversation_partners: HashMap::new(),
        }
    }

    pub fn add_session_sender_if_not_exceeded(
        &mut self,
        username: &String,
        messages_sender: crossbeam_channel::Sender<Message>,
        maximum_sessions_allowed: i32,
    ) -> AddSessionResult {
        match self.conversation_partners.get_mut(username) {
            None => {
                // create a new conversation partner
                let mut conversation_partner: ConversationPartner = ConversationPartner::new();
                conversation_partner
                    .opened_sessions_senders
                    .push(messages_sender);
                // register the new conversation partner
                self.conversation_partners
                    .insert(username.clone(), conversation_partner);
                Success
            }
            Some(conversation_partner) => {
                if conversation_partner.opened_sessions_senders.len() as i32
                    >= maximum_sessions_allowed
                {
                    TooManySessions { messages_sender }
                } else {
                    conversation_partner
                        .opened_sessions_senders
                        .push(messages_sender);
                    Success
                }
            }
        }
    }

    pub fn remove_session_sender(
        &mut self,
        username: &String,
        messages_sender: &crossbeam_channel::Sender<Message>,
    ) {
        match self.conversation_partners.get_mut(username) {
            None => {}
            Some(conversation_partner) => {
                conversation_partner
                    .opened_sessions_senders
                    .retain(|s| !s.same_channel(&messages_sender));
            }
        }
    }

    pub fn add_message_to_private_conversation(
        &mut self,
        sender: String,
        receiver: String,
        content: String,
    ) -> PrivateMessageServerMetadata {
        let private_message = PrivateMessage::new(true, content);
        let mut private_message_server_metadata = PrivateMessageServerMetadata {
            id: 0,
            server_time: private_message.server_time.clone(),
        };

        match self.conversation_partners.get_mut(&sender) {
            None => {
                let private_conversation = PrivateConversation {
                    messages: Some(PrivateMessages {
                        id_offset: 0,
                        messages: vec![private_message],
                    }),
                    show_count: 0,
                    show_count_to_following_messages_count: HashMap::new(),
                };
                // if the conversation partner doesn't exist yet, we should create him
                self.conversation_partners.insert(
                    sender,
                    ConversationPartner {
                        private_conversations: HashMap::from([(receiver, private_conversation)]),
                        opened_sessions_senders: vec![],
                    },
                );
                //TODO add the conversation to the partner (messages list should be None).
            }
            Some(partner) => {
                // when the conversation partner is found first we should find out if he is the owner of the conversation or not
                match partner.private_conversations.get_mut(&receiver) {
                    None => {
                        //TODO if he is not the owner we assume that his conversation partner is the owner
                    }
                    Some(private_conversation) => match &mut private_conversation.messages {
                        None => {}
                        Some(private_messages) => {
                            private_messages.messages.push(private_message);
                            private_message_server_metadata.id = private_messages.id_offset
                                + private_messages.messages.len() as u32;
                        }
                    },
                }
            }
        }
        private_message_server_metadata
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
