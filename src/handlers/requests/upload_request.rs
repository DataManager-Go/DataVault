use actix_web::{
    dev::Payload,
    error::{ErrorBadGateway, ErrorBadRequest},
    Error, FromRequest, HttpRequest,
};
use futures::future::Ready;
use serde::{Deserialize, Serialize};

use crate::{
    handlers::authentication::Authenticateduser,
    models::file::{self, File},
    response_code::RestError,
    utils,
};

#[derive(Debug, Deserialize, Clone)]
pub struct UploadRequest {
    // Required fields
    #[serde(rename = "type")]
    #[serde(with = "upload_type_dser")]
    pub upload_type: UploadType,
    #[serde(rename = "name")]
    pub name: String,

    // Optional fields
    pub url: Option<String>,
    #[serde(rename = "pb")]
    pub public: Option<bool>,
    #[serde(rename = "pbname")]
    pub public_name: Option<String>,
    #[serde(rename = "e")]
    pub encryption: Option<i8>,
    #[serde(rename = "compr")]
    pub compressed: Option<bool>,
    #[serde(rename = "arved")]
    pub archived: Option<bool>,
    #[serde(rename = "r")]
    pub replace_file_by_id: Option<i32>,
    #[serde(rename = "ren")]
    pub replace_equal_names: bool,
    #[serde(rename = "a")]
    pub all: bool,
    #[serde(rename = "attr")]
    pub attributes: Option<FileAttributes>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileAttributes {
    #[serde(rename = "tags")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "groups")]
    pub groups: Option<Vec<String>>,
    #[serde(rename = "ns")]
    pub namespace: String,
}

#[derive(PartialEq, Debug, Clone)]
pub enum UploadType {
    File,
    Url,
}

impl UploadType {
    pub fn encode(&self) -> u8 {
        match self {
            UploadType::File => 0,
            UploadType::Url => 1,
        }
    }

    pub fn decode(i: u8) -> Option<Self> {
        match i {
            0 => Some(Self::File),
            1 => Some(Self::Url),
            _ => None,
        }
    }
}

impl FromRequest for UploadRequest {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let res = || {
            let data_header = req
                .headers()
                .get("Request")
                .ok_or_else(|| ErrorBadRequest("Missing header"))?
                .to_str()
                .map(String::from)
                .map_err(|_| ErrorBadRequest("Malformed header"))?;

            let up_req = serde_json::from_slice::<UploadRequest>(
                &base64::decode(data_header).map_err(|_| ErrorBadGateway("Bad header"))?,
            )
            .map_err(|_| ErrorBadRequest("Bad json"))?;

            Ok(up_req)
        };

        futures::future::ready(res())
    }
}

impl Into<File> for UploadRequest {
    fn into(self) -> File {
        File {
            // Select random string as name if not provided
            name: if self.name.is_empty() {
                utils::random_string(20)
            } else {
                self.name
            },
            encryption: self.encryption.unwrap_or(0) as i32,
            is_public: self.public.unwrap_or(false),
            public_filename: self.public_name,
            local_name: utils::random_string(30),
            ..file::File::default()
        }
    }
}

fn is_encryption_valid(encr: i8) -> Option<bool> {
    match encr {
        0 => None,
        1 | 2 => Some(true),
        _ => Some(false),
    }
}

impl UploadRequest {
    pub fn validate(&self, _user: &Authenticateduser) -> Result<(), RestError> {
        if !is_encryption_valid(self.encryption.unwrap_or(0)).unwrap_or(true) {
            return Err(RestError::IllegalOperation);
        }

        if self.replace_equal_names && self.replace_file_by_id.is_some() {
            return Err(RestError::IllegalOperation);
        }

        // TODO implement user permissions
        match self.upload_type {
            UploadType::File => {}
            UploadType::Url => {}
        }

        Ok(())
    }
}

// Serialize/Deserialize TouchpadOption
mod upload_type_dser {
    use serde::{self, Deserialize, Deserializer, Serializer};

    use super::UploadType;

    pub fn serialize<S>(upload_type: &UploadType, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_u8(upload_type.encode())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<UploadType, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(UploadType::decode(u8::deserialize(deserializer)?).unwrap_or(UploadType::File))
    }
}
