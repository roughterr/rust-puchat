use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct PrivateConversationPartnersHashmapKey {
    pub partner1: String,
    pub partner2: String,
}

/// In the future we might want to change the implementation. That's why we need this function.
pub fn compare_usernames(partner1: &String, partner2: &String) -> bool {
    partner1 < partner2
}

impl PartialEq for PrivateConversationPartnersHashmapKey {
    fn eq(&self, other: &Self) -> bool {
        // Ensure equality regardless of the order of partners
        (self.partner1 == other.partner1 && self.partner2 == other.partner2)
            // || (self.partner1 == other.partner2 && self.partner2 == other.partner1)
    }
}

impl Eq for PrivateConversationPartnersHashmapKey {}

impl Hash for PrivateConversationPartnersHashmapKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Ensure hash is order-independent by hashing sorted pair
        let mut partners = [&self.partner1, &self.partner2];
        partners.sort();
        partners[0].hash(state);
        partners[1].hash(state);
    }
}

#[test]
fn test_private_conversation_partners() {
    let key1 = PrivateConversationPartnersHashmapKey {
        partner1: "Alice".to_string(),
        partner2: "Bob".to_string(),
    };

    let key2 = PrivateConversationPartnersHashmapKey {
        partner1: "Bob".to_string(),
        partner2: "Alice".to_string(),
    };

    let key3 = PrivateConversationPartnersHashmapKey {
        partner1: "Bob".to_string(),
        partner2: "Greg".to_string(),
    };

    let mut map1 = HashMap::new();
    let mut map2 = HashMap::new();

    map1.insert(key1, "Chat between Alice and Bob");
    assert_eq!(map1.get(&key2), Some(&"Chat between Alice and Bob"));

    map2.insert(key3, "Chat between Alice and Bob");
    assert_ne!(map2.get(&key2), Some(&"Chat between Alice and Bob"));
}
