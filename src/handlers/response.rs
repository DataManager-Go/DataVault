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
