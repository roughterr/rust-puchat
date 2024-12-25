use chrono::Utc;

// utility functions

/// returns the current time in milliseconds as a String
pub fn current_time_millis_as_string() -> String {
    // Get the current time in UTC
    let now = Utc::now();
    // Convert to milliseconds since the UNIX epoch
    let millis = now.timestamp_millis();
    // Convert the milliseconds to a string
    millis.to_string()
}