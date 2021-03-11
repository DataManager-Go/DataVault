use chrono::prelude::*;
use serde::Serialize;

use super::requests::upload_request::FileAttributes;

#[derive(Debug, Serialize)]
pub struct StringResponse {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct VecResponse<T>
where
    T: Serialize,
{
    pub slice: Vec<T>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    #[serde(rename = "fileID")]
    pub file_id: i32,
    #[serde(rename = "filename")]
    pub file_name: String,
    #[serde(rename = "publicFilename")]
    pub public_file_name: Option<String>,
    pub checksum: String,
    #[serde(rename = "size")]
    pub file_size: i64,
    #[serde(rename = "ns")]
    pub namespace: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FileItemResponse {
    pub id: i32,
    pub size: i64,
    #[serde(rename = "creation")]
    pub creation_date: DateTime<Utc>,
    pub name: String,
    #[serde(rename = "isPub")]
    pub is_public: bool,
    #[serde(rename = "pubname")]
    pub public_name: String,
    #[serde(rename = "attrib")]
    pub attributes: FileAttributes,
    #[serde(rename = "e")]
    pub encryption: String,
    #[serde(rename = "checksum")]
    pub checksum: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FileListResponse {
    pub files: Vec<FileItemResponse>,
}
