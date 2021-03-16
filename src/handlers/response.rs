use chrono::prelude::*;
use serde::Serialize;

use crate::models::file::File;

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

impl From<File> for UploadResponse {
    fn from(file: File) -> Self {
        UploadResponse {
            namespace: "".to_string(),
            file_size: file.file_size,
            checksum: file.checksum,
            public_file_name: file.public_filename,
            file_name: file.name,
            file_id: file.id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BulkPublishResponse {
    pub files: Vec<UploadResponse>,
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
    pub encryption: i32,
    #[serde(rename = "checksum")]
    pub checksum: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FileListResponse {
    pub files: Vec<FileItemResponse>,
}

impl From<File> for FileItemResponse {
    fn from(file: File) -> FileItemResponse {
        FileItemResponse {
            id: file.id,
            size: file.file_size,
            creation_date: file.uploaded_at,
            name: file.name,
            is_public: file.is_public,
            public_name: file.public_filename.unwrap_or_default(),
            encryption: file.encryption,
            checksum: file.checksum,
            attributes: FileAttributes {
                groups: None,
                tags: None,
                // TODO find a way to also return the namespace
                namespace: "TODO".to_string(),
            },
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct IDsResponse {
    pub ids: Vec<i32>,
}
