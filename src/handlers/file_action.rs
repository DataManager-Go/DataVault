use super::{
    authentication::Authenticateduser,
    requests::{file::FileRequest, upload_request::FileAttributes},
};
use crate::{
    config::Config,
    models::{file::File, namespace::Namespace},
    response_code::{RestError, Success, SUCCESS},
    DbConnection, DbPool,
};

use actix_web::web::{self, Json};

/// Endpoint for registering new users
pub async fn ep_file_action(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    action: web::Path<String>,
    request: Json<FileRequest>,
    user: Authenticateduser,
) -> Result<Json<Success>, RestError> {
    validate_action_request(&request, &action)?;

    // Select files
    let pool_clone = pool.clone();
    let request_clone = request.clone();
    let user_clone = user.clone();
    let files = web::block(move || find_files(&pool_clone, &request_clone, &user_clone)).await??;

    if files.is_empty() {
        return Err(RestError::NotFound);
    }

    if files.len() > 1 && !request.all {
        return Err(RestError::MultipleFilesMatch);
    }

    // TODO actually executing the action
    run_action(&action, files, &request, &pool.get()?, &config).await?;

    Ok(SUCCESS)
}

async fn run_action(
    action: &str,
    files: Vec<File>,
    request: &FileRequest,
    db: &DbConnection,
    config: &Config,
) -> Result<(), RestError> {
    match action {
        "get" => (),
        "update" => (),
        "delete" => {
            for file in files {
                file.delete(db, config).await?;
            }
        }
        "publish" => (),
        _ => unreachable!(),
    };
    Ok(())
}

/// Get the files to update based on
/// the request that was made
fn find_files(
    pool: &web::Data<DbPool>,
    request: &FileRequest,
    user: &Authenticateduser,
) -> Result<Vec<File>, RestError> {
    // Whether to search for a single certain file or by name

    Ok(if request.file_id > 0 {
        // FileID provided, only do the file_action for this single file

        vec![File::find_by_id(
            &pool.get()?,
            request.file_id,
            user.user.id,
        )?]
    } else {
        // FileName provided, find all matching files

        let ns = get_namespace(pool.get()?, &user, &request.attributes)?;

        // Build search file
        let search_file = File {
            user_id: user.user.id,
            id: request.file_id,
            namespace_id: ns.id,
            name: request.name.clone().unwrap_or_default(),
            ..File::default()
        };

        search_file.search(&pool.get()?, false)?
    })
}

/// Validate the file action request and return a namespace,
/// if no file was given by id
fn validate_action_request(request: &FileRequest, action: &str) -> Result<(), RestError> {
    if request.name.as_ref().map(|i| i.len()).unwrap_or_default() == 0 && request.file_id <= 0 {
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

/// Get the requested namespace
fn get_namespace(
    db: DbConnection,
    user: &Authenticateduser,
    attributes: &FileAttributes,
) -> Result<Namespace, RestError> {
    // Use default namespace if none or "default" is provided
    if attributes.namespace.is_empty() || Namespace::is_default_name(&attributes.namespace) {
        return Ok(user.default_ns.as_ref().ok_or(RestError::NotFound)?.clone());
    }

    let ns = attributes.namespace.clone();
    let uid = user.user.id;
    Ok(Namespace::find_by_name(&db, &ns, uid)?.ok_or(RestError::NotFound)?)
}
