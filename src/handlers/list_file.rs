use super::{authentication::Authenticateduser, requests::file::FileList, response};
use crate::{
    models::{
        attribute::{Attribute, AttributeType},
        file::File,
    },
    response_code::RestError,
    DbPool,
};

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
        .map(|(file, namespace, attr)| -> response::FileItemResponse {
            let mut res: response::FileItemResponse = file.into();
            res.attributes.namespace = namespace.name;
            let (tags, groups): (Vec<Attribute>, Vec<Attribute>) = attr
                .into_iter()
                .partition(|i| i.type_.eq(&AttributeType::Tag));

            if !tags.is_empty() {
                res.attributes.tags = Some(tags.into_iter().map(|i| i.name).collect());
            }

            if !groups.is_empty() {
                res.attributes.groups = Some(groups.into_iter().map(|i| i.name).collect());
            }

            res
        })
        .collect();

    Ok(Json(FileListResponse { files: found }))
}
