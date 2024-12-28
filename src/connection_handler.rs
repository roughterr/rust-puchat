use tungstenite::Message;

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
        username: String,
        ///message content
        content: String,
    },
}