use super::{authentication::get_bearer_token, response::StringResponse};
use crate::response_code::RestError;
use actix_web::web::{HttpRequest, Json};

/// Endpoint for registering new users
pub async fn ep_ping(req: HttpRequest) -> Result<Json<StringResponse>, RestError> {
    let content = match get_bearer_token(&req) {
        Some(_) => "Authorized pong",
        None => "pong",
    }
    .to_string();

    Ok(Json(StringResponse { content }))
}
