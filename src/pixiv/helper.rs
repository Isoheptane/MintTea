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