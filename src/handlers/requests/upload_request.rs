use actix_web::{
    dev::Payload,
    error::{ErrorBadGateway, ErrorBadRequest},
    Error, FromRequest, HttpRequest,
};
use futures::future::Ready;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    // Required fields
    #[serde(rename = "type")]
    pub upload_type: u8,
    #[serde(rename = "name")]
    pub name: String,

    // Optional fields
    pub url: Option<String>,
    #[serde(rename = "pb")]
    pub public: Option<bool>,
    #[serde(rename = "pbname")]
    pub public_name: Option<String>,
    #[serde(rename = "e")]
    pub encryption: Option<String>,
    #[serde(rename = "compr")]
    pub compressed: Option<bool>,
    #[serde(rename = "arved")]
    pub archived: Option<bool>,
    #[serde(rename = "r")]
    pub replace_file_by_id: Option<u32>,
    #[serde(rename = "ren")]
    pub replace_equal_names: bool,
    #[serde(rename = "a")]
    pub all: bool,
    #[serde(rename = "attr")]
    pub attributes: Option<FileAttributes>,
}

#[derive(Debug, Deserialize)]
pub struct FileAttributes {
    #[serde(rename = "tags")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "groups")]
    pub groups: Option<Vec<String>>,
    #[serde(rename = "ns")]
    pub namespace: String,
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
                .ok_or(ErrorBadRequest("Missing header"))?
                .to_str()
                .map(String::from)
                .map_err(|_| ErrorBadRequest("Malformed header"))?;

            let up_req = serde_json::from_slice::<UploadRequest>(
                &base64::decode(data_header).map_err(|_| ErrorBadGateway("Bad header"))?,
            )
            .map_err(|e| {
                println!("{:#?}", e);
                ErrorBadRequest("Bad json")
            })?;

            Ok(up_req)
        };

        futures::future::ready(res())
    }
}
