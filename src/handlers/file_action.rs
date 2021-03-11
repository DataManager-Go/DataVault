use super::{authentication::Authenticateduser, requests::file::FileRequest, response};
use crate::{
    models::{file::File, namespace::Namespace},
    response_code::RestError,
    DbConnection, DbPool,
};

use actix_web::web::{self, Json};
use response::FileListResponse;

/// Endpoint for registering new users
pub async fn ep_file_action(
    pool: web::Data<DbPool>,
    web::Path(action): web::Path<String>,
    request: Json<FileRequest>,
    user: Authenticateduser,
) -> Result<Json<FileListResponse>, RestError> {
    validate_action_request(&request, &action)?;

    let mut files: Vec<File> = vec![];

    let ns = if request.file_id > 0 {
        // FileID provided, only do the file_action for this single file
        let fid = request.file_id;
        let db = pool.get()?;
        let file = web::block(move || File::find_by_id(&db, fid)).await?;
        let f_ns = file.namespace_id;
        let db = pool.get()?;
        files.push(file);
        web::block(move || Namespace::find_by_id(&db, f_ns)).await?
    } else {
        // Filename provided, find all matching files
        get_namespace(pool.get()?, &user, &request).await?
    };

    if files.is_empty() {
        return Err(RestError::NotFound);
    }

    if files.len() > 1 && !request.all {
        return Err(RestError::MultipleFilesMatch);
    }

    Ok(Json(FileListResponse { files: vec![] }))
}

// Validate the file action request and return a namespace,
// if no file was given by id
fn validate_action_request(request: &FileRequest, action: &str) -> Result<(), RestError> {
    if request.name.is_empty() && request.file_id <= 0 {
        return Err(RestError::BadRequest);
    }

    if request.all && action == "get" {
        return Err(RestError::NotAllowed);
    }

    match action {
        "delete" | "update" | "get" | "publish" => (),
        _ => return Err(RestError::NotAllowed),
    };

    Ok(())
}

async fn get_namespace(
    db: DbConnection,
    user: &Authenticateduser,
    request: &FileRequest,
) -> Result<Namespace, RestError> {
    // Use default namespace if none or "default" is provided
    if request.attributes.namespace.is_empty()
        || Namespace::is_default_name(&request.attributes.namespace)
    {
        return Ok(user
            .default_ns
            .as_ref()
            .ok_or_else(|| RestError::NotFound)?
            .clone());
    }

    let ns = request.attributes.namespace.clone();
    let uid = user.user.id;
    Ok(web::block(move || Namespace::find_by_name(&db, &ns, uid))
        .await?
        .ok_or_else(|| RestError::NotFound)?)
}
