use super::{
    authentication::Authenticateduser,
    chunked::ChunkedReadFile,
    requests::{file::FileRequest, upload_request::FileAttributes},
};
use crate::{
    config::Config,
    models::{file::File, namespace::Namespace},
    response_code::{RestError, Success, SUCCESS},
    DbConnection, DbPool,
};

use actix_web::web::HttpResponse;
use actix_web::web::{self, Json};
use async_std::path::Path;

/// Endpoint for running a file action
pub async fn ep_file_action(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    action: web::Path<String>,
    request: Json<FileRequest>,
    user: Authenticateduser,
) -> Result<Json<Success>, RestError> {
    validate_action_request(&request)?;

    match action.as_str() {
        "delete" | "update" | "publish" => (),
        _ => return Err(RestError::NotAllowed),
    };

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

    run_action(&action, files, &pool.get()?, &config).await?;

    Ok(SUCCESS)
}

/// Endpoint for downloading a file
pub async fn ep_file_download(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    request: Json<FileRequest>,
    user: Authenticateduser,
) -> Result<HttpResponse, RestError> {
    validate_action_request(&request)?;

    if request.all {
        return Err(RestError::IllegalOperation);
    }

    // Select files
    let pool_clone = pool.clone();
    let request_clone = request.clone();
    let user_clone = user.clone();
    let files = web::block(move || find_files(&pool_clone, &request_clone, &user_clone)).await??;

    if files.is_empty() {
        return Err(RestError::NotFound);
    }

    if files.len() > 1 {
        return Err(RestError::MultipleFilesMatch);
    }

    // We only have this one file
    let file = &files[0];

    // Open local file
    let f = std::fs::File::open(Path::new(&config.server.file_output_path).join(&file.local_name))?;
    let reader = ChunkedReadFile::new(f.metadata()?.len(), 0, f);

    // Build response
    let mut response = HttpResponse::Ok();
    let mut response = response
        .insert_header(("X-Filename", file.name.as_str()))
        .insert_header(("Checksum", file.checksum.as_str()))
        .insert_header(("X-FileID", file.id))
        .insert_header(("ContentLength", file.file_size));

    if file.encryption > 0 {
        response = response.insert_header(("X-Encryption", file.encryption));
    }

    Ok(response.streaming(reader))
}

async fn run_action(
    action: &str,
    files: Vec<File>,
    // request: &FileRequest,
    db: &DbConnection,
    config: &Config,
) -> Result<(), RestError> {
    // TODO implement functions
    match action {
        "update" => (),
        "delete" => delete_files(db, config, files).await?,
        "publish" => (),
        _ => unreachable!(),
    };
    Ok(())
}

/// Delete multiple files
async fn delete_files(
    db: &DbConnection,
    config: &Config,
    files: Vec<File>,
) -> Result<(), RestError> {
    for file in files {
        file.delete(db, config).await?;
    }
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
fn validate_action_request(request: &FileRequest) -> Result<(), RestError> {
    if request.name.as_ref().map(|i| i.len()).unwrap_or_default() == 0 && request.file_id <= 0 {
        return Err(RestError::BadRequest);
    }

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
