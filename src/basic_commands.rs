use std::sync::Arc;

use frankenstein::client_reqwest::Bot;
use frankenstein::types::Message;

use crate::handler::HandlerResult;
use crate::helper::bot_actions;
use crate::helper::message_utils::{message_chat_sender, message_command};
use crate::shared::SharedData;

pub const COMMAND_LIST: &[(&'static str, &'static str)] = &[
    ("help", "顯示幫助信息"),
    ("exit", "退出當前的功能"),
];

pub async fn basic_command_handler(bot: &Bot, data: &Arc<SharedData>, msg: &Message) -> HandlerResult {
    let command = message_command(&msg);
    if let Some(command) = command {
        match command.as_str() {
            "exit" => {
                data.chat_state_storage.release_state(message_chat_sender(&msg)).await;
                return Ok(std::ops::ControlFlow::Break(()))
            }
            "help" => {
                bot_actions::send_message(&bot, msg.chat.id, HELP_MSG).await?;
                return Ok(std::ops::ControlFlow::Break(()))
            }
            _ => {
                return Ok(std::ops::ControlFlow::Continue(()))
            }
        }
    }
    return Ok(std::ops::ControlFlow::Continue(()))
}

const HELP_MSG : &'static str = 
"這裡是薄荷茶～ 目前支持這些功能\n\
- /help : 顯示幫助信息\n\
\n\
貼紙轉換和貼紙下載\n\
/sticker_convert : 轉換貼紙、圖片和動圖\n\
/sticker_set_download : 下載貼紙包";
