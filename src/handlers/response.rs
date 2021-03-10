use serde::Serialize;

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
