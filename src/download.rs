use std::path;

use teloxide::prelude::*;
use teloxide::types::FileId;
use teloxide::net::Download;

#[derive(Debug, Clone, Default)]
pub struct FileName {
    pub basename: String,
    pub extension: String
}

impl FileName {
    pub fn new(basename: String, extension: String) -> FileName {
        FileName { basename, extension }
    }
}


pub fn path_to_filename(path: &str) -> Option<FileName> {
    let file_name = path.split('/').last()?;
    let split: Vec<&str> = file_name.split('.').collect();
    let extension = split.last()?;
    let basename = &split[0..(split.len() - 1)].join(".");
    return Some(FileName::new(basename.to_string(), extension.to_string()));
}

pub async fn download_file(
    bot: Bot,
    file_id: FileId
) -> Result<(Vec<u8>, FileName), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let file_info = bot.get_file(file_id).await?;
    let file_name = path_to_filename(&file_info.path).unwrap_or_default();
    let mut file_content = Vec::<u8>::new();
    file_content.reserve(file_info.size as usize);
    bot.download_file(&file_info.path, &mut file_content).await?;
    Ok((file_content, file_name))
}