use std::sync::Arc;

use frankenstein::{AsyncTelegramApi, methods::SendMessageParams};
use frankenstein::types::{ChatShared, KeyboardButton, KeyboardButtonRequestChat, KeyboardButtonRequestUsers, Message, MessageOrigin, ReplyKeyboardMarkup, ReplyMarkup, SharedUser};
use uuid::Uuid;

use crate::helper::{param_builders, name_utils};
use crate::helper::message_utils::{get_chat_sender, get_command};
use crate::handler::ModalHandlerResult;
use crate::context::{Context, ModalState};
use crate::helper::log::LogOp;
use crate::helper::param_builders::reply_keyboard_remove;
use crate::monitor::MonitorModalState;
use crate::monitor::rules::{FilterRule, MonitorRule};


#[derive(Debug, Clone)]
pub enum SenderInfo {
    SharedUser(SharedUser),
    IdName((i64, Option<String>))
}

impl SenderInfo {
    pub fn id(&self) -> i64 {
        match self {
            SenderInfo::SharedUser(shared_user) => shared_user.user_id as i64,
            SenderInfo::IdName((id, _)) => id.to_owned(),
        }
    }
    pub fn shown_name(&self) -> Option<String> {
        match self {
            SenderInfo::SharedUser(user) => {
                name_utils::user_name(
                    user.first_name.as_deref(), 
                    user.last_name.as_deref(), 
                    user.username.as_deref()
                )
            }
            SenderInfo::IdName((_, name)) => name.to_owned()
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChatInfo {
    ChatShared(ChatShared),
    IdName((i64, Option<String>))
}

impl ChatInfo {
    pub fn id(&self) -> i64 {
        match self {
            ChatInfo::ChatShared(chat_shared) => chat_shared.chat_id,
            ChatInfo::IdName((id, _)) => id.to_owned(),
        }
    }
    pub fn shown_name(&self) -> Option<String> {
        match self {
            ChatInfo::ChatShared(chat_shared) => {
                name_utils::chat_name(
                    chat_shared.title.as_deref(),
                    chat_shared.username.as_deref()
                )
            }
            ChatInfo::IdName((_, name)) => name.to_owned(),
        }
    }
}



pub async fn into_add_rule_modal(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    log::info!(target: "monitor_command", "{} Requested add monitor rule", LogOp(&msg));

    to_wait_user_state(ctx, msg).await?;
    Ok(())
}

pub async fn into_add_rule_forawrd_modal(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    log::info!(target: "monitor_command", "{} Requested add monitor rule by forward message", LogOp(&msg));

    to_wait_forward_state(ctx, msg).await?;
    Ok(())
}

pub async fn into_add_rule_reply_modal(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    log::info!(target: "monitor_command", "{} Requested add monitor rule by reply message", LogOp(&msg));

    to_wait_reply_state(ctx, msg).await?;
    Ok(())
}

pub async fn add_rule_modal_handler(ctx: Arc<Context>, msg: Arc<Message>, state: MonitorModalState) -> ModalHandlerResult {
    match state {
        MonitorModalState::WaitForward => {
            handler_wait_forward_state(ctx, msg).await?;
        }
        MonitorModalState::WaitReply => {
            handler_wait_reply_state(ctx, msg).await?;
        }
        MonitorModalState::WaitUserSelect => {
            handle_wait_user_state(ctx, msg).await?;
        },
        MonitorModalState::WaitChatSelect(sender) => {
            handle_wait_chat_state(ctx, msg, sender).await?;
        }
        MonitorModalState::WaitKeyword(sender, chat) => {
            handle_wait_keyword_state(ctx, msg, sender, chat).await?;
        }
    }
    Ok(())
}

async fn handler_wait_forward_state(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {

    let Some(origin) = msg.forward_origin.as_ref() else {
        ctx.bot.send_message(&build_message_with_markup(
            msg.chat.id,
            "這條消息似乎不是轉發的消息呢……請轉發一條要監視的用戶的消息\n如果需要退出，使用指令 /exit 退出",
            reply_keyboard_remove()
        )).await?;
        return Ok(());
    };
    let (id, name) = match origin.as_ref() {
        MessageOrigin::User(user) => {
            let name = name_utils::user_name(
                Some(user.sender_user.first_name.as_str()), 
                user.sender_user.last_name.as_deref(),
                user.sender_user.username.as_deref()
            );
            (user.sender_user.id as i64, name)
        }
        MessageOrigin::Chat(chat) => {
            let name = name_utils::chat_name(
                chat.sender_chat.title.as_deref(), 
                chat.sender_chat.username.as_deref()
            );
            (chat.sender_chat.id, name)
        },
        MessageOrigin::Channel(channel) => {
            let name = name_utils::chat_name(
                channel.chat.title.as_deref(), 
                channel.chat.username.as_deref()
            );
            (channel.chat.id, name)
        },
        MessageOrigin::HiddenUser(_) => {
            ctx.bot.send_message(&build_message_with_markup(
                msg.chat.id,
                "這條消息的發送者信息被隱藏了呢……請重新轉發一條要監視的用戶的消息\n如果需要退出，使用指令 /exit 退出",
                reply_keyboard_remove()
            )).await?;
            return Ok(())
        },
    };

    log::info!(target: "monitor_add_rule_modal", "{} Received forward message, requesting chat info", LogOp(&msg));

    to_wait_chat_state(ctx, msg, Some(SenderInfo::IdName((id, name)))).await?;

    Ok(())
}

async fn handler_wait_reply_state(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {

    let Some(external_reply) = msg.external_reply.as_ref() else {
        ctx.bot.send_message(&build_message_with_markup(
            msg.chat.id,
            "這條消息似乎不是在其它群組中回覆的消息呢……\n具體來講，您需要在要監視的群組中回覆要監視的用戶，然後選擇「在另一個聊天中回覆」，並在這裡回覆任意內容。\n如果需要退出，使用指令 /exit 退出",
            reply_keyboard_remove()
        )).await?;
        return Ok(());
    };

    let external_reply = external_reply.as_ref();

    let (sender_id, sender_name) = match &external_reply.origin {
        MessageOrigin::User(user) => {
            let name = name_utils::user_name(
                Some(user.sender_user.first_name.as_str()), 
                user.sender_user.last_name.as_deref(),
                user.sender_user.username.as_deref()
            );
            (user.sender_user.id as i64, name)
        }
        MessageOrigin::Chat(chat) => {
            let name = name_utils::chat_name(
                chat.sender_chat.title.as_deref(), 
                chat.sender_chat.username.as_deref()
            );
            (chat.sender_chat.id, name)
        },
        MessageOrigin::Channel(channel) => {
            let name = name_utils::chat_name(
                channel.chat.title.as_deref(), 
                channel.chat.username.as_deref()
            );
            (channel.chat.id, name)
        },
        MessageOrigin::HiddenUser(_) => {
            ctx.bot.send_message(&build_message_with_markup(
                msg.chat.id,
                "這條消息的發送者信息被隱藏了呢……請重新回覆一條消息。\n如果需要退出，使用指令 /exit 退出",
                reply_keyboard_remove()
            )).await?;
            return Ok(())
        }
    };

    let chat = match &external_reply.chat {
        Some(chat) => Some((chat.id, name_utils::chat_name(chat.title.as_deref(), chat.username.as_deref()))),
        None => None
    };

    if chat.is_none() {
        ctx.bot.send_message(&build_message_with_markup(
            msg.chat.id,
            "這條消息轉發的消息中似乎沒有群組的信息，所以規則設置為不對群組進行篩選。",
            reply_keyboard_remove()
        )).await?;
    }

    log::info!(target: "monitor_add_rule_modal", "{} Received reply message, requesting keyword info", LogOp(&msg));

    to_wait_keyword_state(
        ctx, msg, 
        Some(SenderInfo::IdName((sender_id, sender_name))), 
        chat.map(|(id, name)| ChatInfo::IdName((id, name)))
    ).await?;

    Ok(())
}

async fn handle_wait_user_state(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    if get_command(&msg).is_some_and(|s| s == "skip") {
        log::info!(target: "monitor_add_rule_modal", "{} Skipped user info, requesting chat info", LogOp(&msg));
        to_wait_chat_state(ctx, msg, None).await?;
    } else if let Some(shared_user)= msg.users_shared.as_ref().and_then(|u| u.users.get(0).cloned()) {
        log::info!(target: "monitor_add_rule_modal", "{} Received user info, requesting chat info", LogOp(&msg));
        to_wait_chat_state(ctx, msg, Some(SenderInfo::SharedUser(shared_user))).await?;
    } else {
        ctx.bot.send_message(&build_message_with_markup(
            msg.chat.id,
            "請點擊下方的按鈕，選擇一個要監視的用戶～\n如果不需要根據用戶篩選，使用指令 /skip 跳过\n如果需要退出，使用指令 /exit 退出",
            reply_keyboard_remove()
        )).await?;
    }
    Ok(())
}

async fn handle_wait_chat_state(ctx: Arc<Context>, msg: Arc<Message>, sender: Option<SenderInfo>) -> anyhow::Result<()> {
    if get_command(&msg).is_some_and(|s| s == "skip") {
        if sender.is_none() {
            log::info!(target: "monitor_add_rule_modal", "{} Skipped both user info and chat info, exit add rule modal", LogOp(&msg));
            to_exit(ctx, msg, "至少需要選擇監視一個用戶或一個群組。\n如果需要重新開始添加監視規則，使用指令 /monitor").await?;
        } else {
            log::info!(target: "monitor_add_rule_modal", "{} Received chat info, requesting keywords", LogOp(&msg));
            to_wait_keyword_state(ctx, msg, sender, None).await?;
        }
    } else if let Some(chat_shared)= msg.chat_shared.as_ref() {
        let chat_shared = *chat_shared.clone();
        log::info!(target: "monitor_add_rule_modal", "{} Received user info, requesting chat info", LogOp(&msg));
        to_wait_keyword_state(ctx, msg, sender, Some(ChatInfo::ChatShared(chat_shared))).await?;
    } else {
        ctx.bot.send_message(&build_message_with_markup(
            msg.chat.id,
            "請點擊下方的按鈕，請選擇一個要監視的群組～\n如果不需要根據群組篩選，使用指令 /skip 跳过\n如果需要退出，使用指令 /exit 退出",
            reply_keyboard_remove()
        )).await?;
    }
    Ok(())
}

async fn handle_wait_keyword_state(ctx: Arc<Context>, msg: Arc<Message>, sender: Option<SenderInfo>, chat: Option<ChatInfo>) -> anyhow::Result<()> {
    if get_command(&msg).is_some_and(|s| s == "skip") {
        log::info!(target: "monitor_add_rule_modal", "{} Skipped keyword, adding rules", LogOp(&msg));
        to_finish(ctx, msg, sender, chat, vec![]).await?;
        return Ok(());
    }
    let Some(text) = msg.text.as_ref() else {
        ctx.bot.send_message(&build_message_with_markup(
            msg.chat.id,
            "請發送以空格分隔的關鍵詞，總字符數量不超過 64 字。\n如果不需要根據關鍵詞篩選，使用指令 /skip 跳过",
            reply_keyboard_remove()
        )).await?;
        return Ok(());
    };
    let keywords: Vec<String> = text.split_whitespace().into_iter()
        .map(|s| s.to_string())
        .collect();
    log::info!(target: "monitor_add_rule_modal", "{} Received keywords, adding rules", LogOp(&msg));
    to_finish(ctx, msg, sender, chat, keywords).await?;
    Ok(())
}

/*
    Functions for transition to other state with messages
*/

async fn to_wait_forward_state(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    ctx.bot.send_message(&build_message_with_markup(
        msg.chat.id, 
        "請轉發一條要監視的用戶的消息～", 
        reply_keyboard_remove(),
    )).await?;
    ctx.modal_states.set_state(get_chat_sender(&msg), ModalState::Monitor(MonitorModalState::WaitForward)).await;
    Ok(())
}

async fn to_wait_reply_state(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    ctx.bot.send_message(&build_message_with_markup(
        msg.chat.id, 
        "請在這裡回覆一條「要監視的用戶」在「要監視的群組」發送的消息～", 
        reply_keyboard_remove(),
    )).await?;
    ctx.modal_states.set_state(get_chat_sender(&msg), ModalState::Monitor(MonitorModalState::WaitReply)).await;
    Ok(())
}

async fn to_wait_user_state(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    ctx.bot.send_message(&build_message_with_markup(
        msg.chat.id, 
        "請選擇一個要監視的用戶～\n如果不需要根據用戶篩選，使用指令 /skip 跳过", 
        user_request_markup()
    )).await?;
    ctx.modal_states.set_state(get_chat_sender(&msg), ModalState::Monitor(MonitorModalState::WaitUserSelect)).await;
    Ok(())
}

async fn to_wait_chat_state(ctx: Arc<Context>, msg: Arc<Message>, sender: Option<SenderInfo>) -> anyhow::Result<()> {
    ctx.bot.send_message(&build_message_with_markup(
        msg.chat.id, 
        "請選擇一個要監視的群組～\n如果不需要根據群組篩選，使用指令 /skip 跳过", 
        group_request_markup()
    )).await?;
    ctx.modal_states.set_state(get_chat_sender(&msg), ModalState::Monitor(MonitorModalState::WaitChatSelect(sender))).await;
    Ok(())
}

async fn to_wait_keyword_state(ctx: Arc<Context>, msg: Arc<Message>, sender: Option<SenderInfo>, chat: Option<ChatInfo>) -> anyhow::Result<()> {
    ctx.bot.send_message(&build_message_with_markup(
        msg.chat.id,
        "請發送以空格分隔的關鍵詞，總字符數量不超過 64 字。\n如果不需要根據關鍵詞篩選，使用指令 /skip 跳过",
        reply_keyboard_remove()
    )).await?;
    ctx.modal_states.set_state(get_chat_sender(&msg), ModalState::Monitor(MonitorModalState::WaitKeyword(sender, chat))).await;
    Ok(())
}

async fn to_finish(
    ctx: Arc<Context>, 
    msg: Arc<Message>, 
    sender: Option<SenderInfo>, 
    chat: Option<ChatInfo>, 
    keywords: Vec<String>
) -> anyhow::Result<()> {

    let uuid = Uuid::new_v4();

    let finish_message = build_finish_message(sender.as_ref(), chat.as_ref(), keywords.as_slice(), &uuid);

    let sender_name = sender.as_ref().and_then(|sender| sender.shown_name());
    let chat_title = chat.as_ref().and_then(|chat| chat.shown_name());
    
    let filter_rule = FilterRule {
        sender_id: sender.as_ref().map(|u| u.id()),
        chat_id: chat.as_ref().map(|c| c.id()),
        keywords,
    };

    // Monitor can be applied to a chat, but only by administrators
    let rule = MonitorRule {
        uuid,
        filter: filter_rule,
        forward_to: msg.chat.id,
        sender_name,
        chat_title
    };

    log::debug!(
        target: "monitor_add_modal", "{} Adding monitor rule {}", 
        LogOp(&msg), uuid
    );

    ctx.monitor.ruleset.add_rule(Arc::new(rule), uuid);
    let ctx_cloned = ctx.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = ctx_cloned.monitor.ruleset.write_file(ctx_cloned.data_root_path.join("monitor_rules.json")) {
            log::warn!(
                target: "monitor_filesave", "Failed to save monitor rule file{e}"
            );
        }
    });
    ctx.modal_states.release_state(get_chat_sender(&msg)).await;

    ctx.bot.send_message(&build_message_with_markup(msg.chat.id, &finish_message, param_builders::reply_keyboard_remove())).await?;

    Ok(())
}

fn build_finish_message(
    sender: Option<&SenderInfo>,
    chat: Option<&ChatInfo>,
    keywords: &[String], rule_uuid: &Uuid,
) -> String {
    let mut lines: Vec<String> = vec![];
    lines.push(format!("創建監視規則: <code>{}</code>", rule_uuid));
    match sender.map(|inner| (inner.id(), inner.shown_name())) {
        Some((id, None)) => lines.push(format!(" - 用戶:　{}", id)),
        Some((id, Some(name))) => lines.push(format!(" - 用戶:　{} ({})", id, name)),
        None => lines.push(" - 用戶: (不匹配用戶)".to_string())
    }
    match chat.map(|inner| (inner.id(), inner.shown_name())) {
        Some((id, None)) => lines.push(format!(" - 群組: {}", id)),
        Some((id, Some(name))) => lines.push(format!(" - 群組: {} ({})", id, name)),
        None => lines.push(" - 用戶: (不匹配群組)".to_string())
    }
    if keywords.is_empty() {
        lines.push(" -　關鍵詞: (不匹配關鍵詞)".to_string())
    } else {
        lines.push(format!(" - 關鍵詞: {}", keywords.join(", ")));
    }

    lines.join("\n").to_string()
}

async fn to_exit(ctx: Arc<Context>, msg: Arc<Message>, text: &str) -> anyhow::Result<()> {
    ctx.bot.send_message(&build_message_with_markup(
        msg.chat.id,
        text,
        reply_keyboard_remove(),
    )).await?;
    ctx.modal_states.release_state(get_chat_sender(&msg)).await;
    Ok(())
}

fn user_request_markup() -> ReplyMarkup {
    let button_req_user = KeyboardButtonRequestUsers::builder()
        .request_id(0)
        .request_name(true)
        .build();
    let button = KeyboardButton::builder()
        .request_users(button_req_user)
        .text("選擇用戶")
        .build();
    let markup = ReplyKeyboardMarkup::builder()
        .keyboard(vec![vec![button]])
        .one_time_keyboard(true)
        .resize_keyboard(true)
        .build();
    ReplyMarkup::ReplyKeyboardMarkup(markup)
}

fn group_request_markup() -> ReplyMarkup {
    let button_req_chat = KeyboardButtonRequestChat::builder()
        .request_id(0)
        .chat_is_channel(false)
        .request_title(true)
        .build();
    let button = KeyboardButton::builder()
        .request_chat(button_req_chat)
        .text("選擇群組")
        .build();
    let markup = ReplyKeyboardMarkup::builder()
        .keyboard(vec![vec![button]])
        .one_time_keyboard(true)
        .resize_keyboard(true)
        .build();
    ReplyMarkup::ReplyKeyboardMarkup(markup)
}

fn build_message_with_markup(chat_id :i64, text :&str, markup: ReplyMarkup) -> SendMessageParams {
    SendMessageParams::builder()
        .chat_id(chat_id)
        .parse_mode(frankenstein::ParseMode::Html)
        .text(text)
        .reply_markup(markup)
        .build()
}