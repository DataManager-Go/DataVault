use super::{authentication::Authenticateduser, requests::file::FileList, response};
use crate::{models::file::File, response_code::RestError, DbPool};

use actix_web::web::{self, Json};
use response::FileListResponse;

/// Endpoint for registering new users
pub async fn ep_list_files(
    pool: web::Data<DbPool>,
    request: Json<FileList>,
    user: Authenticateduser,
) -> Result<Json<FileListResponse>, RestError> {
    let found = File::search(&pool.get()?, &request, user.user.clone())?
        .into_iter()
        .map(|(file, namespace)| -> response::FileItemResponse {
            let mut res: response::FileItemResponse = file.into();
            res.attributes.namespace = namespace.name;
            res
        })
        .collect();

    Ok(Json(FileListResponse { files: found }))
}
