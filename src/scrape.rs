use reqwest::Url;

use crate::file::FileContent;

#[derive(Debug)]
pub struct Conclusion {
    pub final_url: Url,
    pub extension: String,
    pub content: FileContent,
}
