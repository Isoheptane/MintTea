use std::sync::LazyLock;

use regex::Regex;

use crate::pixiv::types::{IllustRequest, SendMode};

static PIXIV_LINK_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| 
    Regex::new(r"^(?:(?:https?:\/\/)?(?:www\.)?(?:pixiv\.net\/)(?:(?:en\/)?artworks\/|i\/|member_illust\.php\?illust_id=))?([0-9]+)(?:[#\?].*)?")
    .expect("Pixiv Link+ID regex construct failed.")
);

pub enum PixivCommandParseResult {
    Success(IllustRequest),
    InvalidId,
    ShowHelp
}

pub fn parse_pixiv_command(text: &str) -> PixivCommandParseResult {
    let args: Vec<&str> = text.split_whitespace().collect();

    let Some(raw_input) = args.get(1) else {
        return PixivCommandParseResult::ShowHelp;
    };
    if *raw_input == "help" {
        return PixivCommandParseResult::ShowHelp;
    }

    let Some((_, [id_str])) = PIXIV_LINK_ID_REGEX.captures(raw_input).map(|c| c.extract()) else {
        return PixivCommandParseResult::InvalidId;
    };
    let Ok(id) = u64::from_str_radix(&id_str, 10) else {
        return PixivCommandParseResult::InvalidId;
    };

    let mut no_page_limit = false;
    let mut files_mode = false;
    let mut archive_mode = false;
    for arg in args.iter().skip(2) {
        if *arg == "nolim" { no_page_limit = true; }
        if *arg == "archive" { archive_mode = true; }
        if *arg == "files" { files_mode = true; }
    }

    let send_mode = match (files_mode, archive_mode) {
        (false, false) => SendMode::Photos,
        (true, false) => SendMode::Files,
        (false, true) => SendMode::Archive,
        (true, true) => SendMode::Archive
    };

    let req = IllustRequest {
        id,
        no_page_limit,
        silent_page_limit: false,
        send_mode
    };

    return PixivCommandParseResult::Success(req)
}

/* Pixiv Link Parser */

static PIXIV_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| 
    Regex::new(r"^(?:(?:https?:\/\/)?(?:www\.)?(?:pixiv\.net\/)(?:(?:en\/)?artworks\/|i\/|member_illust\.php\?illust_id=))([0-9]+)(?:[#\?].*)?")
    .expect("Pixiv Link regex construct failed.")
);

pub enum PixivLinkParseResult {
    Success(u64),
    InvalidId,
    NotMatch
}

pub fn parse_pixiv_link(text: &str) -> PixivLinkParseResult {
    let Some((_, [id_str])) = PIXIV_LINK_REGEX.captures(&text).map(|c| c.extract()) else {
        return PixivLinkParseResult::NotMatch;
    };
    let Ok(id) = u64::from_str_radix(&id_str, 10) else {
        return PixivLinkParseResult::InvalidId;
    };
    return PixivLinkParseResult::Success(id);
}

/* FANBOX Parser */

static FANBOX_LINK_REGEX: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"^(?:https?:\/\/)?(?:([a-zA-Z0-9-]+)\.)?fanbox\.cc(?:\/@([a-zA-Z0-9-]+))?(?:\/posts\/?([0-9]+)?)?(?:[#?&].*)?$")
    .expect("Fanbox Link regex construct failed.")
);

pub enum FanboxLinkParseResult {
    Success{ name: String, post_id: Option<u64> },
    EmptyName,
    InvalidPostId,
    NotMatch
}

pub fn parse_fanbox_link(text: &str) -> FanboxLinkParseResult {
    
    let Some(capture) = FANBOX_LINK_REGEX.captures(&text) else {
        return FanboxLinkParseResult::NotMatch;
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
        return FanboxLinkParseResult::EmptyName;
    };

    let post_id_str = capture.get(3).map(|m| m.as_str());
    let post_id = match post_id_str {
        Some(post_id_str) => {
            let Ok(post_id) = u64::from_str_radix(&post_id_str, 10) else {
                return FanboxLinkParseResult::InvalidPostId;
            };
            Some(post_id)
        },
        None => None,
    };
    
    return FanboxLinkParseResult::Success { name: name.to_string(), post_id };
}