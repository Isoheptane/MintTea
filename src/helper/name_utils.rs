
pub fn user_name(first_name: Option<&str>, last_name: Option<&str>, username: Option<&str>) -> Option<String> {
    match (first_name, last_name, username) {
        (Some(first), None, _) => Some(first.to_string()),
        (Some(first ), Some(last), _) => Some(format!("{} {}", first, last)),
        (_, _, Some(username)) => Some(username.to_string()),
        (_, _, None) => None,
    }
}

pub fn chat_name(title: Option<&str>, username: Option<&str>) -> Option<String> {
    match (title, username) {
        (Some(title), _) => Some(title.to_string()),
        (None, Some(username)) => Some(username.to_string()),
        (None, None) => None
    }
}