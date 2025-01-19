use once_cell::sync::Lazy;
use std::collections::HashMap;

/**
* Define the map as a global static variable.
*/
static USER_TO_PASSWORD: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("ian", "ian");
    map.insert("dan", "dan");
    map.insert("chris", "chris");
    map
});

pub fn are_credentials_correct(username: &str, password: &str) -> bool {
    println!(
        "are_credentials_correct called. username: {}, password: {}",
        username, password
    );
    match USER_TO_PASSWORD.get(username) {
        Some(&password_from_db) => password_from_db.eq(password),
        None => false,
    }
}
