use serde::Deserialize;

use super::upload_request::FileAttributes;
#[derive(Clone, Debug, Deserialize)]
pub struct FileList {
    #[serde(rename = "fid")]
    pub file_id: i32,
    pub name: String,
    #[serde(rename = "allns")]
    pub all_namespaces: bool,
    pub order: Option<String>,
    pub attributes: FileAttributes,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FileRequest {
    #[serde(rename = "fid")]
    pub file_id: i32,
    pub name: Option<String>,
    #[serde(rename = "pubname")]
    pub public_name: Option<String>,
    // updates: FileUpdateItem,
    pub all: bool,
    pub attributes: FileAttributes,
}
