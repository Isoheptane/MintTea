use std::sync::Arc;

use frankenstein::types::Message;

use crate::helper::bot_actions;
use crate::helper::log::LogOp;
use crate::pixiv::types::FanboxCreatorGetResponse;
use crate::context::Context;

pub async fn fanbox_to_kemono_handler(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    name: String,
    post_id: Option<u64>
) -> anyhow::Result<()> {

    let api_url = format!("https://api.fanbox.cc/creator.get?creatorId={}", name);

    log::info!(
        target: "fanbox_to_kemono",
        "{} Requesting pixiv API: {}",
        LogOp(&msg), api_url
    );

    let request = ctx.pixiv.client.get(api_url)
        .header("Origin", "https://www.fanbox.cc");

    let response: FanboxCreatorGetResponse = request.send().await?.json().await?;

    // NOTICE:
    // Fanbox returns general error when requesting a non-existing artist
    let Some(response_body) = response.body else {

        let error_msg = match response.error.as_ref() {
            Some(msg) => msg.as_str(),
            None => "<no error message>",
        };

        log::info!(
            target: "fanbox_to_kemono",
            "{} Response body is empty, error message: {}",
            LogOp(&msg), error_msg
        );

        bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, "似乎沒找到這個創作者呢…… (也有可能是查詢 FANBOX 創作者失敗了)",
            msg.message_id, None
        ).await?;

        return Ok(())
    };

    // It should be number, but we keep it a string here
    let user_id = response_body.user.user_id;

    let kemono_link = match post_id {
        Some(post_id) => format!("https://kemono.cr/fanbox/user/{user_id}/post/{post_id}"),
        None => format!("https://kemono.cr/fanbox/user/{user_id}"),
    };

    bot_actions::send_reply_message(
        &ctx.bot, msg.chat.id, 
        format!("可能的 kemono.cr 連結： {}", kemono_link),
        msg.message_id, None
    ).await?;

    Ok(())
}