pub mod dto;
pub mod util;
mod connection_handler;
mod user_context;
pub mod private_conversation_partners;
// if we do not do this, we won't be able to see src/dto.rs in src/bin/simple-client.rs, for example