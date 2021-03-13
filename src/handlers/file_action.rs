use super::{authentication::Authenticateduser, chunked::ChunkedReadFile, requests::file::FileRequest, response::{UploadResponse, BulkPublishResponse}};
use crate::{
    config::Config,
    models::file::File,
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
        "delete" | "update" => (),
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

    run_action(&action, files, pool.get()?, &config, &request).await?;

    Ok(SUCCESS)
}

/// Endpoint for publishing files
pub async fn ep_publish_file(
    pool: web::Data<DbPool>,
    request: Json<FileRequest>,
    user: Authenticateduser,
) -> Result<Json<BulkPublishResponse>, RestError> {
    validate_action_request(&request)?;

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

    let files = publish_files(&pool.get()?, files, request.public_name.as_ref().cloned().unwrap_or_default())?;

    Ok(Json(BulkPublishResponse{
        files,
    }))
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

/// Run the actual file action
async fn run_action(
    action: &str,
    files: Vec<File>,
    db: DbConnection,
    config: &Config,
    request: &FileRequest,
) -> Result<(), RestError> {
    // TODO implement functions
    match action {
        "update" => (),
        "delete" => delete_files(&db, config, files).await?,
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

        // Simply find the file by its ID
        vec![File::find_by_id(
            &pool.get()?,
            request.file_id,
            user.user.id,
        )?]
    } else {
        // FileName provided, find all matching files

        // Pick ns
        let ns = super::utils::retrieve_namespace(&pool.get()?, &Some(&request.attributes), &user)?;

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

/// Publish multiple files
fn publish_files(db: &DbConnection, files: Vec<File>, public_name: String)->Result<Vec<UploadResponse>, RestError>{
    let mut publishes: Vec<UploadResponse> = Vec::new();

    if files.len() == 1 && files[0].is_public {
        return Err(RestError::AlreadyPublic);
    }

    if files.len() > 1 && !public_name.is_empty(){
        return Err(RestError::AlreadyPublic);
    }

    for mut file in files.into_iter() {
        if file.is_public{
            continue;
        }

        file.publish(&db, &public_name)?;
        publishes.push(file.into());
    }

    Ok(publishes)
}

/// Validate the file action request and return a namespace,
/// if no file was given by id
fn validate_action_request(request: &FileRequest) -> Result<(), RestError> {
    // Either a files name or id has to be passed
    if request.name.as_ref().map(|i| i.len()).unwrap_or_default() == 0 && request.file_id <= 0 {
        return Err(RestError::BadRequest);
    }

    Ok(())
}
