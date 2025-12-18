use crate::pixiv::config::PixivConfig;
use crate::pixiv::types::IllustInfo;

pub fn have_spoiler(pixiv_config :&PixivConfig, info: &IllustInfo) -> bool {
    let nsfw = info.tags.contains_tag("R-18");
    let r18g = info.tags.contains_tag("R-18G");
    (pixiv_config.spoiler_r18g && r18g) || (pixiv_config.spoiler_nsfw && (nsfw || r18g))
}

pub fn illust_caption(info: &IllustInfo, page: Option<u64>) -> String {

    let nsfw = info.tags.contains_tag("R-18");
    let r18g = info.tags.contains_tag("R-18G");

    let prefix = match (nsfw, r18g) {
        (false, false) => "",
        (true, false) => "#NSFW ",
        (_, true) => "#NSFW #R18G "
    };

    let page_num_str = match page {
        Some(page) => format!(" ({}/{})", page, info.page_count),
        None => "".to_string()
    };

    format!(
        "{prefix}<a href=\"https://www.pixiv.net/artworks/{}\">{}</a> / <a href=\"https://www.pixiv.net/users/{}\">{}</a>{page_num_str}",
        info.id, info.title, info.author_id, info.author_name,
    )
}

pub fn illust_caption_detailed(info: &IllustInfo) -> String {
let mut text: String = String::new();
    text.push_str(&format!("<b>{}</b>\n", info.title));
    if !info.description.is_empty() {
        let replaced_desc = info.description.replace("<br />", "\n");
        text.push_str(&format!("<blockquote expandable>{}</blockquote>\n\n", replaced_desc));
    }

    let author_url = format!("https://www.pixiv.net/users/{}", info.author_id);
    text.push_str(&format!("Artist: <a href=\"{}\">{}</a>\n", author_url, info.author_name));

    let mut tag_list: String = String::new();
    for tag in info.tags.tags.iter() {
        // Try get en
        let en_translation = match tag.translation.as_ref() {
            Some(translations) => {
                translations.get("en")
            },
            None => None,
        };
        let tag_str = tag.tag.replace("-", "");
        let tag_text = match en_translation {
            Some(en_translation) => format!("#{} ({})  ", tag_str, en_translation),
            None => format!("#{}  ", tag_str),
        };
        tag_list.push_str(&tag_text);
    }
    text.push_str(&format!("Tags: {}\n", tag_list));

    let source_url = format!("https://www.pixiv.net/artworks/{}", info.id);
    text.push_str(&format!("Source: {}\n", source_url));

    return text;
}