pub mod attribute;
pub mod file;
pub mod upload_request;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct NamespaceRequest {
    #[serde(rename = "ns")]
    pub name: String,
    #[serde(rename = "newName")]
    pub new_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CredentialsRequest {
    pub username: String,
    #[serde(rename = "mid")]
    pub machine_id: Option<String>,
    #[serde(rename = "pass")]
    pub password: String,
}

impl CredentialsRequest {
    // Returns true if one value is empty
    pub fn has_empty(&self) -> bool {
        self.username.is_empty() || self.password.is_empty()
    }
}

#[derive(Debug, Deserialize)]
pub struct StatsRequest {
    #[serde(rename = "ns")]
    pub namespace: Option<String>,
}
