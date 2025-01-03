use crate::user_context::AddSessionResult::{Success, TooManySessions};
use chrono::{DateTime, Utc};
use crate::private_conversation_partners::{
    compare_usernames, PrivateConversationPartnersHashmapKey,
};
use std::collections::HashMap;
use std::hash::Hash;
use tungstenite::Message;

/// Metadata that the server add to a private message after the server receives the message.
pub struct PrivateMessageServerMetadata {
    /// message id
    pub id: u32,
    /// the datetime when the message was registered on the server.
    pub server_time: DateTime<Utc>,
}

/// Represents a private chat message in the server internal memory.
struct PrivateMessage {
    ///true - user 1 is the author. false - user 2 is the author
    is_sender_user1: bool,
    /// the content of the message.
    content: String,
    /// the time (in milliseconds) when the server received the message.
    server_time: DateTime<Utc>,
    /// True for deleted messages.
    is_deleted: bool,
}

impl PrivateMessage {
    pub fn new(is_sender_user1: bool, content: String) -> Self {
        PrivateMessage {
            is_sender_user1,
            content,
            server_time: Utc::now(),
            is_deleted: false,
        }
    }
}

/// Contains data related to the private conversation and these data are relevant to both
/// conversation partners.
struct PrivateConversation {
    /// Defines from which index the array "messages" start. The default value must be 0.
    id_offset: u32,
    /// A list of messages.
    messages: Vec<PrivateMessage>,
}

impl PrivateConversation {
    pub fn new(first_message: PrivateMessage) -> Self {
        PrivateConversation {
            id_offset: 0,
            messages: vec![first_message],
        }
    }
}

/// Contains data related to the private conversation but these data are relevant only to one of the
/// two conversation partners.
struct PrivateConversationOnePartnerSpecificData {
    /// how many times this conversation has been shown to the user
    show_count: u32,
    /// HashMap where the key is the number of times when the user retrieved the conversation.
    /// And the value is the number of messages that were sent as follower messages to this show count.
    /// It is done to prevent a situation when the user sends a few consecutive messages but the fist
    /// message is lost. We do not want the user to see the second or any other consecutive messages
    /// until the previous are delivered.
    show_count_to_following_messages_count: HashMap<u32, u16>,
}

impl PrivateConversationOnePartnerSpecificData {
    pub fn new(first_message_content: String) -> Self {
        PrivateConversationOnePartnerSpecificData {
            show_count: 0,
            show_count_to_following_messages_count: HashMap::new(),
        }
    }
}

/// The data of one user.
pub struct ChatUser {
    /// the key is the user id of the Conversation partner.
    pub private_conversations: HashMap<String, PrivateConversationOnePartnerSpecificData>,
    // the currently opened sessions of the users.
    pub opened_sessions_senders: Vec<crossbeam_channel::Sender<Message>>,
}

impl ChatUser {
    pub fn new() -> Self {
        ChatUser {
            private_conversations: HashMap::new(),
            opened_sessions_senders: Vec::new(),
        }
    }
}

/// The data about all users.
pub struct ApplicationScope {
    pub chat_users: HashMap<String, ChatUser>,
    pub private_conversations: HashMap<PrivateConversationPartnersHashmapKey, PrivateConversation>,
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
            chat_users: HashMap::new(),
            private_conversations: HashMap::new(),
        }
    }

    pub fn add_session_sender_if_not_exceeded(
        &mut self,
        username: &String,
        messages_sender: crossbeam_channel::Sender<Message>,
        maximum_sessions_allowed: i32,
    ) -> AddSessionResult {
        match self.chat_users.get_mut(username) {
            None => {
                // create a new conversation partner
                let mut chat_user: ChatUser = ChatUser::new();
                chat_user.opened_sessions_senders.push(messages_sender);
                // register the new conversation partner
                self.chat_users.insert(username.clone(), chat_user);
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
        match self.chat_users.get_mut(username) {
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
        let is_sender_partner1: bool = compare_usernames(&sender, &receiver);
        let new_private_message = PrivateMessage::new(is_sender_partner1.clone(), content);
        let server_time = new_private_message.server_time.clone();
        let (partner1, partner2) = if is_sender_partner1 {
            (sender, receiver)
        } else {
            (receiver, sender)
        };
        let private_conversation_partners =
            PrivateConversationPartnersHashmapKey { partner1, partner2 };
        match self
            .private_conversations
            .get_mut(&private_conversation_partners)
        {
            None => {
                self.private_conversations.insert(
                    private_conversation_partners,
                    PrivateConversation::new(new_private_message),
                );
                PrivateMessageServerMetadata { id: 1, server_time }
            }
            Some(private_messages) => {
                private_messages.messages.push(new_private_message);
                PrivateMessageServerMetadata {
                    id: private_messages.id_offset + private_messages.messages.len() as u32,
                    server_time,
                }
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
