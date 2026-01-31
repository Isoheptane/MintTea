use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone)]
pub struct KemonoCommandParam {
    pub as_telegraph: bool,
    pub as_media: bool,
    pub as_archive: bool,
}

impl KemonoCommandParam {
    pub fn link_default() -> KemonoCommandParam {
        KemonoCommandParam {
            as_telegraph: true,
            as_media: false,
            as_archive: false,   
        }
    }
}

#[derive(Debug, Clone)]
pub struct KemonoRequest {
    pub service: String,
    pub user_id: String,
    pub post_id: String,
    pub param: KemonoCommandParam
}

#[derive(Debug, Clone)]
pub struct FanboxRequest {
    pub username: String,
    pub post_id: Option<String>,
    pub param: KemonoCommandParam
}

pub enum KemonoCommandParseResult {
    Kemono(KemonoRequest),
    Fanbox(FanboxRequest),
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
    let param = KemonoCommandParam {
        as_telegraph,
        as_media,
        as_archive,
    };

    if let Some((service, user_id, post_id)) = parse_kemono_link(*raw_input) {
        return KemonoCommandParseResult::Kemono(KemonoRequest { service, user_id, post_id, param });
    } else if let Some((username, post_id)) = parse_fanbox_link(*raw_input) {
        return KemonoCommandParseResult::Fanbox(FanboxRequest { username, post_id, param });
    } else {
        return KemonoCommandParseResult::InvalidLink;
    }
}

/* Kemono Parser */

static KEMONO_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| 
    Regex::new(r"^(?:https?:\/\/)?kemono\.cr\/([a-zA-Z]+)\/user\/([0-9]+)\/post\/([0-9]+)(?:[#\?].*)?$")
    .expect("kemono.cr regex construct failed.")
);

pub fn parse_kemono_link(text: &str) -> Option<(String, String, String)> {
    let Some((_, [service, user_id, post_id ])) = KEMONO_LINK_REGEX.captures(&text).map(|c| c.extract()) else {
        return None;
    };
    return Some((service.to_string(), user_id.to_string(), post_id.to_string()));
}

/* FANBOX Parser */

static FANBOX_LINK_REGEX: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"^(?:https?:\/\/)?(?:([a-zA-Z0-9-]+)\.)?fanbox\.cc(?:\/@([a-zA-Z0-9-]+))?(?:\/posts\/?([0-9]+)?)?\/?(?:[#?&].*)?$")
    .expect("Fanbox Link regex construct failed.")
);

pub fn parse_fanbox_link(text: &str) -> Option<(String, Option<String>)> {
    
    let Some(capture) = FANBOX_LINK_REGEX.captures(&text) else {
        return None;
    };

    let name = 'get_name: {
        if let Some(match_name) = capture.get(2) {
            break 'get_name match_name.as_str();
        }
        if let Some(match_subdomain) = capture.get(1) {
            let match_subdomain = match_subdomain.as_str();
            if match_subdomain != "www" && match_subdomain != "api" {
                break 'get_name match_subdomain;
            } 
        }
        return None;
    };

    let post_id_str = capture.get(3).map(|m| m.as_str());

    return Some((name.to_string(), post_id_str.map(|s| s.to_string())));
}