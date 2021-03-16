use super::{
    authentication::Authenticateduser,
    chunked::ChunkedReadFile,
    requests::file::{FileRequest, FileUpdateItem},
    response::{BulkPublishResponse, IDsResponse, UploadResponse},
};
use crate::{
    config::Config,
    models::{file::File, namespace::Namespace},
    response_code::RestError,
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
) -> Result<Json<IDsResponse>, RestError> {
    validate_action_request(&request)?;

    match action.as_str() {
        "delete" => (),
        "update" => {
            if request.updates.is_none() {
                return Err(RestError::BadRequest);
            }
        }
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

    let fids_changed = run_action(&action, files, pool.get()?, &config, &request, &user).await?;

    Ok(Json(IDsResponse { ids: fids_changed }))
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

    let files = publish_files(
        &pool.get()?,
        files,
        request.public_name.as_ref().cloned().unwrap_or_default(),
    )?;

    Ok(Json(BulkPublishResponse { files }))
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
    user: &Authenticateduser,
) -> Result<Vec<i32>, RestError> {
    Ok(match action {
        "update" => update_files(db, files, request.updates.clone().unwrap(), user)?,
        "delete" => delete_files(db, config.to_owned(), files).await?,
        _ => unreachable!(),
    })
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
    db: DbConnection,
    config: Config,
    files: Vec<File>,
) -> Result<Vec<i32>, RestError> {
    web::block(move || {
        for file in files.iter() {
            file.delete(&db, &config)?;
        }

        Ok(files.iter().map(|i| i.id).collect())
    })
    .await?
}

/// Publish multiple files
fn publish_files(
    db: &DbConnection,
    files: Vec<File>,
    public_name: String,
) -> Result<Vec<UploadResponse>, RestError> {
    let mut publishes: Vec<UploadResponse> = Vec::new();

    if files.len() == 1 && files[0].is_public {
        return Err(RestError::AlreadyPublic);
    }

    if files.len() > 1 && !public_name.is_empty() {
        return Err(RestError::AlreadyPublic);
    }

    for mut file in files.into_iter() {
        if file.is_public {
            continue;
        }

        file.publish(&db, &public_name)?;
        publishes.push(file.into());
    }

    Ok(publishes)
}

/// Update given files and returns all
/// fileids of files which were modified
fn update_files(
    db: DbConnection,
    files: Vec<File>,
    update: FileUpdateItem,
    user: &Authenticateduser,
) -> Result<Vec<i32>, RestError> {
    Ok(files
        .into_iter()
        .map(|i| update_file(&db, &i, &update, user).map(|j| if j { i.id } else { 0 }))
        .collect::<Result<Vec<i32>, RestError>>()?
        .into_iter()
        .filter(|i| *i > 0)
        .collect())
}

/// Update a single File and return
/// whether it was modified or not
fn update_file(
    db: &DbConnection,
    file: &File,
    update: &FileUpdateItem,
    user: &Authenticateduser,
) -> Result<bool, RestError> {
    let mut did_update = false;
    let mut file = file.clone();

    // Update namespace
    if let Some(ref new_ns) = update.new_namespace {
        let ns = Namespace::find_by_name(db, &new_ns, user.user.id)?.ok_or(RestError::NotFound)?;
        if ns.id != file.namespace_id {
            // Update ns
            file.namespace_id = ns.id;
            did_update = true;
        }
    }

    // Update filename
    if let Some(ref new_name) = update.new_name {
        if !new_name.is_empty() {
            file.name = new_name.clone();
            did_update = true;
        }
    }

    // Publish / Unpublish file
    if let Some(ref public) = update.is_public {
        if !public.is_empty() {
            if file.public_filename.is_none() {
                // File needs to be shared first
                return Err(RestError::NotPublic);
            }

            file.is_public = public.parse().map_err(|_| RestError::BadRequest)?;
            did_update = true;
        }
    }

    // TODO add/remove attributes

    if did_update {
        file.save(db)?;
    }

    Ok(did_update)
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
