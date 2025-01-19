use crate::dto::NewPrivateMessageSequenceResponse;
use crate::private_conversation_partners::{
    compare_usernames, PrivateConversationPartnersHashmapKey,
};
use crate::user_context::AddSessionResult::{Success, TooManySessions};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
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
    user1_specific_data: PrivateConversationOnePartnerSpecificData,
    user2_specific_data: PrivateConversationOnePartnerSpecificData,
}

impl PrivateConversation {
    pub fn new() -> Self {
        PrivateConversation {
            id_offset: 0,
            messages: vec![],
            user1_specific_data: PrivateConversationOnePartnerSpecificData::new(),
            user2_specific_data: PrivateConversationOnePartnerSpecificData::new(),
        }
    }
}

/// Contains data related to the private conversation but these data are relevant only to one of the
/// two conversation partners.
struct PrivateConversationOnePartnerSpecificData {
    /// id offset of the message sequences
    message_sequence_id_offset: u32,
    /// the fulfillment of the message sequences.
    /// It is done to prevent a situation when the user sends a few consecutive messages but the fist
    /// message is lost. We do not want the user to see the second or any other consecutive messages
    /// until the previous are delivered.
    message_sequence_state: Vec<u32>,
}

impl PrivateConversationOnePartnerSpecificData {
    pub fn new() -> Self {
        PrivateConversationOnePartnerSpecificData {
            message_sequence_id_offset: 0,
            message_sequence_state: Vec::new(),
        }
    }
}

/// The data of one user.
pub struct ChatUser {
    // the currently opened sessions of the users.
    pub opened_sessions_senders: Vec<crossbeam_channel::Sender<Message>>,
}

impl ChatUser {
    pub fn new() -> Self {
        ChatUser {
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
                let mut new_private_conversation = PrivateConversation::new();
                new_private_conversation.messages.push(new_private_message);
                self.private_conversations
                    .insert(private_conversation_partners, new_private_conversation);
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

    pub fn get_new_message_sequence(
        &mut self,
        sender: String,
        receiver: String,
    ) -> NewPrivateMessageSequenceResponse {
        let is_sender_partner1: bool = compare_usernames(&sender, &receiver);
        let (partner1, partner2) = if is_sender_partner1 {
            (sender, receiver.clone())
        } else {
            (receiver.clone(), sender)
        };
        let private_conversation_partners =
            PrivateConversationPartnersHashmapKey { partner1, partner2 };
        match self
            .private_conversations
            .get_mut(&private_conversation_partners)
        {
            None => {
                let mut private_conversation = PrivateConversation::new();
                let response = Self::get_new_message_sequence_from_conversation(
                    is_sender_partner1,
                    &mut private_conversation,
                    receiver.clone(),
                );
                self.private_conversations
                    .insert(private_conversation_partners, private_conversation);
                response
            }
            Some(private_conversation) => Self::get_new_message_sequence_from_conversation(
                is_sender_partner1,
                private_conversation,
                receiver,
            ),
        }
    }

    /// private function
    fn get_new_message_sequence_from_conversation(
        is_sender_partner1: bool,
        private_conversation: &mut PrivateConversation,
        receiver: String,
    ) -> NewPrivateMessageSequenceResponse {
        let one_partner_data: &mut PrivateConversationOnePartnerSpecificData = if is_sender_partner1
        {
            &mut private_conversation.user1_specific_data
        } else {
            &mut private_conversation.user2_specific_data
        };
        one_partner_data.message_sequence_state.push(0);
        // the number 1 is 0
        let sequence_id = one_partner_data.message_sequence_id_offset
            + one_partner_data.message_sequence_state.len() as u32
            - 1;
        NewPrivateMessageSequenceResponse {
            sequence_id,
            receiver_username: receiver,
        }
    }

    pub fn approach_message_sequence(
        &mut self,
        sender: String,
        receiver: String,
        message_sequence_id: u32,
        message_sequence_index: u32,
    ) -> Result<(), String> {
        let is_sender_partner1: bool = compare_usernames(&sender, &receiver);
        let (partner1, partner2) = if is_sender_partner1 {
            (sender, receiver.clone())
        } else {
            (receiver.clone(), sender)
        };
        let private_conversation_partners =
            PrivateConversationPartnersHashmapKey { partner1, partner2 };
        match self
            .private_conversations
            .get_mut(&private_conversation_partners)
        {
            None => Err("the conversation does not exist".to_string()),
            Some(private_conversation) => {
                let mut private_conversation_one_partner_specific_data = if is_sender_partner1 {
                    &mut private_conversation.user1_specific_data
                } else {
                    &mut private_conversation.user2_specific_data
                };
                let index_in_state_arr: usize = (message_sequence_id
                    - private_conversation_one_partner_specific_data.message_sequence_id_offset)
                    as usize;
                if let Some(how_many_messages_already_sent) =
                    private_conversation_one_partner_specific_data
                        .message_sequence_state
                        .get_mut(index_in_state_arr)
                {
                    // check if the next index is the index we intend to extend
                    if *how_many_messages_already_sent + 1 == message_sequence_index {
                        *how_many_messages_already_sent += 1;
                        Ok(())
                    } else {
                        Err(format!(
                            "another index for the sequence with id {} expected",
                            message_sequence_id
                        ))
                    }
                } else {
                    Err(format!(
                        "the sequence with id {} does not exist",
                        message_sequence_id
                    ))
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
