use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateAttribute {
    pub name: String,
    #[serde(rename = "newname")]
    pub new_name: String,
    pub namespace: String,
}
