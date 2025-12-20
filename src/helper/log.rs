use std::fmt::Display;

use chrono::DateTime;
use frankenstein::types::{ChatType, Message};
use owo_colors::OwoColorize;
pub struct LogOp<'a>(pub &'a Message);
impl<'a> Display for LogOp<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = self.0;
        write!(f, "[{}:{}]", msg.chat.id, msg.message_id)
    }
}

#[derive(Debug, Clone)]
pub enum MessageTimestampFormat {
    Time,
    DateTime
}

#[derive(Debug, Clone)]
pub struct MessageTimestampDisplay {
    pub timestamp: i64,
    pub format: MessageTimestampFormat
}

impl MessageTimestampDisplay {
    pub fn time(timestamp: i64) -> Self { Self { timestamp, format: MessageTimestampFormat::Time } }
    pub fn date_time(timestamp: i64) -> Self { Self { timestamp, format: MessageTimestampFormat::DateTime } }
}

impl Display for MessageTimestampDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match DateTime::from_timestamp_secs(self.timestamp) {
            Some(time) => match self.format {
                MessageTimestampFormat::Time => write!(f, "{}", time.format("%T")),
                MessageTimestampFormat::DateTime => write!(f, "{}", time.format("%F %T")),
            }
            None => write!(f, "<invalid time>")
        }
    }
}

pub struct MessageIdentityDisplay<'a>(pub &'a Message);

impl<'a> Display for MessageIdentityDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = self.0;

        if msg.chat.type_field == ChatType::Private {
            
            match msg.chat.first_name.as_ref() {
                Some(first_name) => write!(f, "{}", first_name.green())?,
                None => write!(f, "{}", "<no name>".dimmed())?,
            };
        
            if let Some(last_name) = msg.chat.last_name.as_ref() {
                write!(f, " {}", last_name.green())?;
            }
        
            if let Some(username) = msg.chat.username.as_ref() {
                write!(f, " ({}{})", "@".cyan(), username.cyan())?;
            }

            write!(f, "{}", " @ Private Chat".dimmed())?;
            
        } else {
            // Check sender chat fist (on behalf of group or channel)
            if let Some(sender_chat) = msg.sender_chat.as_ref() {

                match sender_chat.title.as_ref() {
                    Some(title) => write!(f, "{}", title.blue())?,
                    None => write!(f, "{}", "<no title>".dimmed())?,
                    // For channel and groups, they should always have title
                }
                
                write!(f, " (")?;
                if sender_chat.type_field == ChatType::Channel {
                    write!(f, "{}", "Channel".dimmed())?;
                } else {
                    write!(f, "{}", "Group".dimmed())?;
                }
                if let Some(username) = sender_chat.username.as_ref() {
                    write!(f, " {}{}", "@".cyan(), username.cyan())?;
                }
                write!(f, ")")?;

            } else if let Some(user) = msg.from.as_ref() {

                write!(f, "{}", user.first_name.green())?;
                if let Some(last_name) = user.last_name.as_ref() {
                    write!(f, " {}", last_name.green())?;
                }
                if let Some(username) = user.username.as_ref() {
                    write!(f, " ({}{})", "@".cyan(), username.cyan())?;
                }

            }
        
            // Check chat type
            if msg.chat.type_field == frankenstein::types::ChatType::Channel {
                write!(f, "{}", " @ Channel: ".dimmed())?;
            } else {
                write!(f, "{}", " @ Group: ".dimmed())?;
            }
            match msg.chat.title.as_ref() {
                Some(title) => write!(f, "{}", title.dimmed())?,
                None => write!(f, "{}", "<no title>".dimmed())?
            };
        }

        Ok(())
    }
}

pub struct MessageContentDisplay<'a>(pub &'a Message);

impl<'a> Display for MessageContentDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = self.0;
        // Supported message types
        if let Some(reply) = msg.reply_to_message.as_ref() {
            let prefix = "   >   ".dimmed();
            write!(f, "{}", "   > ".dimmed())?;
            write!(f, "{}{}{}", "[".bright_black(), MessageTimestampDisplay::date_time(msg.date as i64), "]".bright_black())?;
            writeln!(f, " {}", MessageIdentityDisplay(&reply))?;
            chat_content_inner_helper(&reply, f, prefix)?;
        }
        chat_content_inner_helper(msg, f, "  ")?;

        Ok(())
    }
}

fn chat_content_inner_helper(msg :&Message, f: &mut std::fmt::Formatter<'_>, prefix: impl std::fmt::Display) -> std::fmt::Result {
    if let Some(sticker) = msg.sticker.as_ref() {
        write!(f, "{prefix}{}{}", "[".dimmed(), "Sticker".yellow())?;
        if let Some(emoji) = sticker.emoji.as_ref() {
            write!(f, " {}", emoji)?;
        }
        writeln!(f, "{}", "]".dimmed())?;
    }
    if let Some(animation) = msg.animation.as_ref() {
        write!(f, "{prefix}{}{}", "[".dimmed(), "Animation".yellow())?;
        if let Some(name) = animation.file_name.as_ref() {
            write!(f, " {}", name.cyan())?;
        }
        write!(f, " {}{}", animation.duration.dimmed(), "s".dimmed())?;
        writeln!(f, "{}", "]".dimmed())?;
    }
    if let Some(audio) = msg.audio.as_ref() {
        write!(f, "{prefix}{}{}", "[".dimmed(), "Audio".yellow())?;
        if let Some(name) = audio.file_name.as_ref() {
            write!(f, " {}", name.cyan())?;
        }
        write!(f, " {}{}", audio.duration.dimmed(), "s".dimmed())?;
        writeln!(f, "{}", "]".dimmed())?;
    }
    if let Some(document) = msg.document.as_ref() {
        write!(f, "{prefix}{}{}", "[".dimmed(), "Document".yellow())?;
        if let Some(name) = document.file_name.as_ref() {
            write!(f, " {}", name.cyan())?;
        }
        writeln!(f, "{}", "]".dimmed())?;
    }
    if let Some(_) = msg.photo.as_ref() {
        writeln!(f, "{prefix}{}{}{}", "[".dimmed(), "Photo".yellow(), "]".dimmed())?;
        // There is few information about a photo
    }
    if let Some(video) = msg.video.as_ref() {
        write!(f, "{prefix}{}{}", "[".dimmed(), "Video".yellow())?;
        if let Some(name) = video.file_name.as_ref() {
            write!(f, " {}", name)?;
        }
        write!(f, " {}{}", video.duration.dimmed(), "s".dimmed())?;
        writeln!(f, "{}", "]".dimmed())?;
    }
    if let Some(video) = msg.video_note.as_ref() {
        write!(f, "{prefix}{}{}", "[".dimmed(), "Video Note".yellow())?;
        write!(f, " {}{}", video.duration.dimmed(), "s".dimmed())?;
        writeln!(f, "{}", "]".dimmed())?;
    }
    if let Some(voice) = msg.voice.as_ref() {
        write!(f, "{prefix}{}{}", "[".dimmed(), "Voice".yellow())?;
        write!(f, " {}{}", voice.duration.dimmed(), "s".dimmed())?;
        writeln!(f, "{}", "]".dimmed())?;
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

pub struct MessageDisplay<'a>(pub &'a Message);

impl<'a> Display for MessageDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = self.0;
        write!(f, "{}{}{}", "[".bright_black(), MessageTimestampDisplay::time(msg.date as i64), "]".bright_black())?;
        write!(f, " {}\n{}", MessageIdentityDisplay(msg), MessageContentDisplay(msg))
    }
}