use super::{authentication::Authenticateduser, requests::upload_request::UploadRequest};
use crate::{
    config::Config,
    response_code::{RestError, Success, SUCCESS},
    DbPool,
};
use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};

use actix_web::web::{self, Json};

/// Endpoint for registering new users
pub async fn ep_list_files(
    _pool: web::Data<DbPool>,
    _config: web::Data<Config>,
    _user: Authenticateduser,
) -> Result<Json<Success>, RestError> {
    Ok(SUCCESS)
}

/// Endpoint for uploading new files
pub async fn ep_upload(
    _pool: web::Data<DbPool>,
    _config: web::Data<Config>,
    user: Authenticateduser,
    upload_request: UploadRequest,
    mut payload: Multipart,
) -> Result<Json<Success>, RestError> {
    upload_request.validate(&user)?;

    println!("ns:{:?}", user.default_namespace);
    // Replace by ID
    // Replace by name

    // Create new

    Ok(SUCCESS)
}
