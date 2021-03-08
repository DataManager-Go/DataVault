use super::authentication::get_bearer_token;
use crate::response_code::RestError;

use actix_web::web::{HttpRequest, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StringResponse {
    content: String,
}

/// Endpoint for registering new users
pub async fn ep_ping(req: HttpRequest) -> Result<Json<StringResponse>, RestError> {
    let content = match get_bearer_token(&req) {
        Some(_) => "Authorized pong",
        None => "pong",
    }
    .to_string();

    Ok(Json(StringResponse { content }))
}
