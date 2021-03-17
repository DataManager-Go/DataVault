use super::upload_request::FileAttributes;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct FileList {
    #[serde(rename = "fid")]
    pub file_id: i32,
    pub name: String,
    #[serde(rename = "allns")]
    pub all_namespaces: bool,
    pub attributes: FileAttributes,
    #[serde(rename = "opt")]
    pub optional: OptionalRequestParameter,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OptionalRequestParameter {
    #[serde(rename = "verb")]
    verbose: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FileRequest {
    #[serde(rename = "fid")]
    pub file_id: i32,
    pub name: Option<String>,
    #[serde(rename = "pubname")]
    pub public_name: Option<String>,
    pub updates: Option<FileUpdateItem>,
    pub all: bool,
    pub attributes: FileAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileUpdateItem {
    #[serde(rename = "ispublic")]
    pub is_public: Option<String>,
    #[serde(rename = "name")]
    pub new_name: Option<String>,
    #[serde(rename = "namespace")]
    pub new_namespace: Option<String>,
    #[serde(rename = "rem_tags")]
    pub remove_tags: Option<Vec<String>>,
    #[serde(rename = "rem_groups")]
    pub remove_groups: Option<Vec<String>>,
    #[serde(rename = "add_tags")]
    pub add_tags: Option<Vec<String>>,
    #[serde(rename = "add_groups")]
    pub add_groups: Option<Vec<String>>,
}
