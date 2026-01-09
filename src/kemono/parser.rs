use std::sync::LazyLock;

use regex::Regex;

pub enum KemonoCommandParseResult {
    Success(String),
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

    if let Some(suffix) = kemono_link_suffix(text) {
        return KemonoCommandParseResult::Success(suffix.to_string());
    } else {
        return KemonoCommandParseResult::InvalidLink;
    }
}

static KEMONO_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| 
    Regex::new(r"^(?:https?:\/\/)?kemono\.cr(\/[a-zA-Z]+\/user\/[0-9]+\/post\/[0-9]+)(?:[#\?].*)?$")
    .expect("kemono.cr regex construct failed.")
);

pub fn kemono_link_suffix(text: &str) -> Option<&str> {
    let Some((_, [suffix])) = KEMONO_LINK_REGEX.captures(&text).map(|c| c.extract()) else {
        return None;
    };
    return Some(suffix);
}