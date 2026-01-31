use std::sync::LazyLock;

use regex::Regex;

use crate::pixiv::types::{IllustRequest, SendMode};

static PIXIV_LINK_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| 
    Regex::new(r"^(?:(?:https?:\/\/)?(?:www\.)?(?:pixiv\.net\/)(?:(?:en\/)?artworks\/|i\/|member_illust\.php\?illust_id=))?([0-9]+)\/?(?:[#\?].*)?$")
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
    let mut detailed_caption = false;
    let mut metadata_only = false;

    for arg in args.iter().skip(2) {
        if *arg == "nolim" { no_page_limit = true; }
        if *arg == "archive" { archive_mode = true; }
        if *arg == "files" { files_mode = true; }
        if *arg == "detail" { detailed_caption = true; }
        if *arg == "metaonly" { metadata_only = true; }
    }

    let send_mode = match (files_mode, archive_mode) {
        (false, false) => SendMode::Photos,
        (true, false) => SendMode::Files,
        (false, true) => SendMode::Archive,
        (true, true) => SendMode::Archive
    };

    let req = IllustRequest {
        id,
        metadata_only,
        no_page_limit,
        send_mode,
        detailed_caption,
    };

    return PixivCommandParseResult::Success(req)
}

/* Pixiv Link Parser */

static PIXIV_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| 
    Regex::new(r"^(?:(?:https?:\/\/)?(?:www\.)?(?:pixiv\.net\/)(?:(?:en\/)?artworks\/|i\/|member_illust\.php\?illust_id=))([0-9]+)\/?(?:[#\?].*)?$")
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