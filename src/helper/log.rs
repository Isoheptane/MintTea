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
            writeln!(f, " > {}", LogChatSource(&reply))?;
            chat_content_inner_helper(&reply, f, " > ")?;
        }
        chat_content_inner_helper(msg, f, "")?;

        Ok(())
    }
}

fn chat_content_inner_helper(msg :&Message, f: &mut std::fmt::Formatter<'_>, prefix: &str) -> std::fmt::Result {
    if let Some(sticker) = msg.sticker.as_ref() {
        write!(f, "{prefix}[Sticker")?;
        if let Some(emoji) = sticker.emoji.as_ref() {
            write!(f, " {}", emoji)?;
        }
        writeln!(f, "]")?;
    }
    if let Some(animation) = msg.animation.as_ref() {
        write!(f, "{prefix}[Animation")?;
        if let Some(name) = animation.file_name.as_ref() {
            write!(f, " {}", name)?;
        }
        write!(f, " {}s", animation.duration)?;
        writeln!(f, "]")?;
    }
    if let Some(audio) = msg.audio.as_ref() {
        write!(f, "{prefix}[Audio")?;
        if let Some(name) = audio.file_name.as_ref() {
            write!(f, " {}", name)?;
        }
        write!(f, " {}s", audio.duration)?;
        writeln!(f, "]")?;
    }
    if let Some(document) = msg.document.as_ref() {
        write!(f, "{prefix}[Document")?;
        if let Some(name) = document.file_name.as_ref() {
            write!(f, " {}", name)?;
        }
        writeln!(f, "]")?;
    }
    if let Some(_) = msg.photo.as_ref() {
        writeln!(f, "{prefix}[Photo]")?;
        // There is few information about a photo
    }
    if let Some(video) = msg.video.as_ref() {
        write!(f, "{prefix}[Video")?;
        if let Some(name) = video.file_name.as_ref() {
            write!(f, " {}", name)?;
        }
        write!(f, " {}s", video.duration)?;
        writeln!(f, "]")?;
    }
    if let Some(video) = msg.video_note.as_ref() {
        write!(f, "{prefix}[Video Note")?;
        write!(f, " {}s", video.duration)?;
        writeln!(f, "]")?;
    }
    if let Some(voice) = msg.voice.as_ref() {
        write!(f, "{prefix}[Voice")?;
        write!(f, " {}s", voice.duration)?;
        writeln!(f, "]")?;
    }
    if let Some(caption) = msg.caption.as_ref() {
        for line in caption.lines() {
            writeln!(f,"{prefix}{line}")?;
        }
    }
    if let Some(text) = msg.text.as_ref() {
        for line in text.lines() {
            writeln!(f,"{prefix}{line}")?;
        }
    }

    Ok(())
}