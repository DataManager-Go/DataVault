use super::{authentication::Authenticateduser, requests::attribute::UpdateAttribute, utils};
use crate::models::attribute::AttributeType;
use crate::{
    response_code::{RestError, Success, SUCCESS},
    DbPool,
};

use actix_web::web::{self, Json};

/// Endpoint for registering new users
pub async fn ep_attribute_action(
    pool: web::Data<DbPool>,
    action: web::Path<(String, String)>,
    request: Json<UpdateAttribute>,
    user: Authenticateduser,
) -> Result<Json<Success>, RestError> {
    let (attr, action) = action.into_inner();

    validate_action(&action)?;
    validate_request(&action, &request)?;

    let db = pool.get()?;
    let namespace =
        web::block(move || utils::retrieve_namespace_by_name(&db, &request.namespace, &user))
            .await??;

    let attr_type = to_attribute(attr)?;

    // TODO execute the requested attribute action

    Ok(SUCCESS)
}

fn validate_request(action: &str, request: &UpdateAttribute) -> Result<(), RestError> {
    if action == "update" && request.new_name.is_empty() {
        return Err(RestError::BadRequest);
    }
    Ok(())
}

fn validate_action(action: &str) -> Result<(), RestError> {
    if matches!(action, "update" | "delete" | "get" | "crate") {
        Ok(())
    } else {
        Err(RestError::BadRequest)
    }
}

fn to_attribute(attr: String) -> Result<AttributeType, RestError> {
    match attr.to_lowercase().as_str() {
        "tag" => Ok(AttributeType::Tag),
        "group" => Ok(AttributeType::Group),
        _ => Err(RestError::BadRequest),
    }
}
