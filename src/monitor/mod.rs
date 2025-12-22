pub mod context;

mod rules;
mod add_rule;
mod parser;

use std::sync::Arc;

use frankenstein::methods::{CopyMessageParams, SendMessageParams};
use frankenstein::AsyncTelegramApi;
use frankenstein::types::Message;
use futures::future::BoxFuture;
use uuid::Uuid;

use crate::helper::{bot_actions, param_builders};
use crate::helper::message_utils::{get_command, get_sender_id};
use crate::handler::{HandlerResult, ModalHandlerResult};
use crate::context::Context;
use crate::helper::log::LogOp;
use crate::monitor::add_rule::{ChatInfo, SenderInfo, add_rule_modal_handler, into_add_rule_forawrd_modal, into_add_rule_modal, into_add_rule_reply_modal};
use crate::monitor::parser::parse_monitor_command;



#[derive(Debug, Clone)]
pub enum MonitorModalState {
    WaitForward,
    WaitReply,
    WaitUserSelect,
    WaitChatSelect(Option<SenderInfo>),
    WaitKeyword(Option<SenderInfo>, Option<ChatInfo>)
}

pub fn monitor_command_handler(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = monitor_command_handler_impl(ctx, msg);
    return Box::pin(fut);
}

async fn monitor_command_handler_impl(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {
    let command = get_command(&msg);
    let Some(text) = msg.text.as_ref() else {
        return Ok(std::ops::ControlFlow::Continue(()))
    };
    if command.is_none() && command.is_some_and(|s| s != "monitor" && s != "mon") {
        return Ok(std::ops::ControlFlow::Continue(()));
    }

    match parse_monitor_command(text) {
        parser::MonitorCommandParseResult::AddRule => {
            into_add_rule_modal(ctx, msg).await?;
            return Ok(std::ops::ControlFlow::Break(()))
        },
        parser::MonitorCommandParseResult::AddRuleByForward => {
            into_add_rule_forawrd_modal(ctx, msg).await?;
            return Ok(std::ops::ControlFlow::Break(()))
        }
        parser::MonitorCommandParseResult::AddRuleByReply => {
            into_add_rule_reply_modal(ctx, msg).await?;
            return Ok(std::ops::ControlFlow::Break(()))
        }
        parser::MonitorCommandParseResult::ListRules => {
            list_rules(ctx, msg).await?;
            return Ok(std::ops::ControlFlow::Break(()))
        }
        parser::MonitorCommandParseResult::RemoveRule(uuid) => {
            if let Some(uuid_result) = uuid {
                if let Ok(uuid) = uuid_result {
                    remove_rule(ctx, msg, uuid).await?;
                } else {
                    bot_actions::send_reply_message(
                        &ctx.bot, msg.chat.id, 
                        "UUID 的格式似乎不太對呢……",
                        msg.message_id, None
                    ).await?;
                }
            } else {
                bot_actions::send_reply_message(
                    &ctx.bot, msg.chat.id, 
                    "請在 /monitor remove 指令後面加上要刪除的規則的 UUID 哦——",
                    msg.message_id, None
                ).await?;
            }
        }
        parser::MonitorCommandParseResult::RemoveAllRule => {
            remove_all_rules(ctx, msg).await?;
        }
        parser::MonitorCommandParseResult::Help => {

        },
        parser::MonitorCommandParseResult::NotMatch => {

        },
    }
    
    Ok(std::ops::ControlFlow::Continue(()))
}

pub async fn list_rules(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    let Some(receiver_id) = get_sender_id(&msg) else {
        bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, 
            "好像找不到你的 ID 呢……", 
            msg.message_id, None
        ).await?;
        return Ok(())
    };
    
    let rules = ctx.monitor.ruleset.get_receiver_rules(receiver_id);

    let mut lines: Vec<String> = vec![];
    lines.push(format!("你有 {} 條監視規則哦：", rules.len()));
    
    for (uuid, rule) in rules {
        lines.push("".to_string());
        lines.push(format!("<code>{}</code>", uuid));
        if let Some(id) = rule.filter.sender_id {
            let nickname_suffix = match rule.sender_name {
                Some(nickname) => format!(" ({})", nickname),
                None => "".to_string(),
            };
            lines.push(format!(" - 用戶: {}{}", id, nickname_suffix));
        }
        if let Some(id) = rule.filter.chat_id {
            let nickname_suffix = match rule.chat_title {
                Some(title) => format!(" ({})", title),
                None => "".to_string(),
            };
            lines.push(format!(" - 群組: {}{}", id, nickname_suffix));
        }
        lines.push(format!(" - 關鍵詞: {}", rule.filter.keywords.join(", ")));
    }

    let params = SendMessageParams::builder()
        .chat_id(msg.chat.id)
        .reply_parameters(param_builders::reply_parameters(msg.message_id, None))
        .parse_mode(frankenstein::ParseMode::Html)
        .text(lines.join("\n"))
        .build();
    ctx.bot.send_message(&params).await?;

    Ok(())
}

pub async fn remove_rule(ctx: Arc<Context>, msg: Arc<Message>, uuid: Uuid) -> anyhow::Result<()> {

    if ctx.monitor.ruleset.remove_rule(&uuid) {
        bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, 
            format!("刪除了一條 UUID 為 {uuid} 的規則——"),
            msg.message_id, None
        ).await?;

        let ctx_cloned = ctx.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = ctx_cloned.monitor.ruleset.write_file(ctx_cloned.data_root_path.join("monitor_rules.json")) {
                log::warn!(
                    target: "monitor_filesave", "Failed to save monitor rule file{e}"
                );
            }
        });

    } else {
        bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, 
            "好像找不到這條規則呢……",
            msg.message_id, None
        ).await?;
    }

    Ok(())
}

pub async fn remove_all_rules(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {

    let rules = ctx.monitor.ruleset.get_receiver_rules_uuid(msg.chat.id);
    let rule_len = rules.len();

    let ctx_cloned = ctx.clone();
    tokio::task::spawn_blocking(move || {

        for uuid in &rules {
            ctx_cloned.monitor.ruleset.remove_rule(uuid);
        }

        if let Err(e) = ctx_cloned.monitor.ruleset.write_file(ctx_cloned.data_root_path.join("monitor_rules.json")) {
            log::warn!(
                target: "monitor_filesave", "Failed to save monitor rule file{e}"
            );
        }
    });

    bot_actions::send_reply_message(
        &ctx.bot, msg.chat.id, 
        format!("刪除了 {} 條規則——", rule_len),
        msg.message_id, None
    ).await?;

    Ok(())
}

/// This is a monitor handler, will always return Continue
pub fn monitor_handler(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = monitor_handler_impl(ctx, msg);
    return Box::pin(fut);
}

async fn monitor_handler_impl(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {

    // This function needs early return
    tokio::spawn(async move {
        monitor_handler_worker(ctx, msg).await
    });

    Ok(std::ops::ControlFlow::Continue(()))
}

async fn monitor_handler_worker(ctx: Arc<Context>, msg: Arc<Message>) {
    
    let forward_to = ctx.monitor.ruleset.check_message(&msg);

    for chat_id in forward_to {
        let ctx = ctx.clone();
        let msg = msg.clone();
        tokio::spawn(async move {

            let param = CopyMessageParams::builder()
                .chat_id(chat_id)
                .from_chat_id(msg.chat.id)
                .message_id(msg.message_id)
                .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
                .build();

            if let Err(e) = ctx.bot.copy_message(&param).await {
                log::warn!(
                    target: "monitor_forward_worker", "{} Failed to make a portal message: {e}", 
                    LogOp(&msg)
                );
            }
        });
    }

}

pub async fn monitor_modal_handler(
    ctx: Arc<Context>, 
    msg: Arc<Message>, 
    state: MonitorModalState
) -> ModalHandlerResult {
    add_rule_modal_handler(ctx, msg, state).await?;
    Ok(())
}