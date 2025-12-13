use std::{any::Any, fmt::Display};

use frankenstein::types::Message;

use crate::sticker;

pub struct LogOp<'a>(pub &'a Message);
impl<'a> Display for LogOp<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = self.0;
        write!(f, "[{}:{}]", msg.chat.id, msg.message_id)
    }
}

pub struct LogChatSource<'a>(pub &'a Message);

impl<'a> Display for LogChatSource<'a> {
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

pub struct LogChatContent<'a>(pub &'a Message);

impl<'a> Display for LogChatContent<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = self.0;
        // Supported message types
        if let Some(reply) = msg.reply_to_message.as_ref() {
            write!(f, " > {}\n", LogChatSource(reply))?;
            let content = LogChatContent(reply).to_string();
            for line in content.lines() {
                write!(f, " > {}\n", line)?;
            }
        }
        if let Some(sticker) = msg.sticker.as_ref() {
            write!(f, "[Sticker")?;
            if let Some(emoji) = sticker.emoji.as_ref() {
                write!(f, " {}", emoji)?;
            }
            write!(f, "]")?;
        }
        if let Some(text) = msg.text.as_ref() {
            write!(f, "{}", text)?;
        }

        Ok(())
    }
}