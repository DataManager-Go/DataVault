use super::{authentication::Authenticateduser, requests::file, response};
use crate::{response_code::RestError, DbPool};

use actix_web::web::{self, Json};
use response::FileListResponse;

/// Endpoint for registering new users
pub async fn ep_list_files(
    _pool: web::Data<DbPool>,
    _request: Json<file::FileList>,
    _user: Authenticateduser,
) -> Result<Json<FileListResponse>, RestError> {
    Ok(Json(FileListResponse { files: vec![] }))
}
