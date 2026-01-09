use std::sync::LazyLock;

use regex::Regex;

pub struct KemonoRequest {
    pub service: String,
    pub user_id: String,
    pub post_id: String,
    pub as_telegraph: bool,
    pub as_media: bool,
    pub as_archive: bool,
}

pub enum KemonoCommandParseResult {
    Success(KemonoRequest),
    InvalidLink,
    ShowHelp,
}

pub fn parse_kemono_command(text: &str) -> KemonoCommandParseResult {
    let args: Vec<&str> = text.split_whitespace().collect();

    let Some(raw_input) = args.get(1) else {
        return KemonoCommandParseResult::ShowHelp;
    };
    if *raw_input == "help" {
        return KemonoCommandParseResult::ShowHelp;
    }

    let Some((service, user_id, post_id)) = kemono_link_suffix(*raw_input) else {
        return KemonoCommandParseResult::InvalidLink;
    };
    
    let mut as_telegraph = false;
    let mut as_media = false;
    let mut as_archive = true;

    if args.len() > 2 {
        as_archive = false;
    }

    for arg in args.iter().skip(2) {
        if *arg == "telegraph" { as_telegraph = true; }
        if *arg == "media" { as_media = true; }
        if *arg == "archive" { as_archive = true; }
    }

    return KemonoCommandParseResult::Success(KemonoRequest {
        service,
        user_id,
        post_id,
        as_telegraph,
        as_media,
        as_archive,
    })

}

static KEMONO_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| 
    Regex::new(r"^(?:https?:\/\/)?kemono\.cr\/([a-zA-Z]+)\/user\/([0-9]+)\/post\/([0-9]+)(?:[#\?].*)?$")
    .expect("kemono.cr regex construct failed.")
);

pub fn kemono_link_suffix(text: &str) -> Option<(String, String, String)> {
    let Some((_, [service, user_id, post_id ])) = KEMONO_LINK_REGEX.captures(&text).map(|c| c.extract()) else {
        return None;
    };
    return Some((service.to_string(), user_id.to_string(), post_id.to_string()));
}