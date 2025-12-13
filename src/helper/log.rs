use std::fmt::Display;

use frankenstein::types::Message;

pub struct LogSource<'a>(pub &'a Message);
impl<'a> Display for LogSource<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = self.0;
        write!(f, "[{}:{}]", msg.chat.id, msg.message_id)
    }
}


pub struct ChatSource<'a>(pub &'a Message);

impl<'a> Display for ChatSource<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = self.0;
        match msg.chat.type_field {
            frankenstein::types::ChatType::Private => {
                write!(f, "[")?;
                match msg.chat.first_name.as_ref() {
                    Some(first_name) => write!(f, "{}", first_name)?,
                    None => write!(f, "<no name>")?
                }
                if let Some(last_name) = msg.chat.last_name.as_ref() {
                    write!(f, " {}", last_name)?;
                }
                if let Some(username) = msg.chat.username.as_ref() {
                    write!(f, " (@{})", username)?;
                }
                write!(f, " @ Private Chat]")?;
            },
            frankenstein::types::ChatType::Group |
            frankenstein::types::ChatType::Supergroup |
            frankenstein::types::ChatType::Channel => {
                write!(f, "[")?;
                if let Some(user) = msg.from.as_ref() {
                    write!(f, "{}", user.first_name)?;
                    if let Some(last_name) = user.last_name.as_ref() {
                        write!(f, " {}", last_name)?;
                    }
                    if let Some(username) = user.username.as_ref() {
                        write!(f, " (@{})", username)?;
                    }
                } else if let Some(sender_chat) = msg.sender_chat.as_ref() {
                    match sender_chat.title.as_ref() {
                        Some(title) => write!(f, "{}", title)?,
                        None => write!(f, "<no title>")?,
                    }
                    if let Some(username) = sender_chat.username.as_ref() {
                        write!(f, " (@{})", username)?;
                    }
                }

                if msg.chat.type_field == frankenstein::types::ChatType::Channel {
                    write!(f, " @ Channel: ")?;
                } else {
                    write!(f, " @ Group: ")?;
                }
                match msg.chat.title.as_ref() {
                    Some(title) => write!(f, "{title}")?,
                    None => write!(f, "<no title>")?
                };
                write!(f, "]")?;
            }
        }

        Ok(())
    }
}