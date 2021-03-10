use super::{authentication::Authenticateduser, requests::upload_request::UploadRequest};
use crate::{
    config::Config,
    models::file::{self, NewFile},
    models::namespace::Namespace,
    response_code::{RestError, Success, SUCCESS},
    utils, DbConnection, DbPool,
};
use actix_multipart::Multipart;

use actix_web::web::{self, Json};
use async_std::{fs, io::prelude::*, path::Path};
use futures::StreamExt;
use lazy_static::__Deref;

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
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    user: Authenticateduser,
    upload_request: UploadRequest,
    payload: Multipart,
) -> Result<Json<Success>, RestError> {
    upload_request.validate(&user)?;

    let db = pool.get()?;

    // Find namespace
    let target_namespace = retrieve_namespace(&upload_request, &user, &db)?;

    let mut file = NewFile {
        name: upload_request.name,
        user_id: user.user.id,
        //encryption: upload_request.encryption.unwrap_or_default(),
        namespace_id: target_namespace.id,
        is_public: upload_request.public.unwrap_or(false),
        public_filename: upload_request.public_name.unwrap_or_default(),
        local_name: utils::random_string(30),
        ..file::NewFile::default()
    };

    let (crc, size, mime_type) =
        multipart_to_file(payload, config.deref(), &file.local_name).await?;

    file.checksum = crc;
    file.file_size = size;
    file.file_type = mime_type;

    println!("{:#?}", file);

    // Replace by ID
    // Replace by name

    // Create new

    Ok(SUCCESS)
}

/// Write a multipart to a given file. Returns
/// (crc32, size, mimeType)
pub async fn multipart_to_file(
    mut part: Multipart,
    config: &Config,
    filename: &str,
) -> Result<(String, i64, String), RestError> {
    // Get first multipart file
    let mut part = part
        .next()
        .await
        .ok_or_else(|| RestError::BadRequest)?
        .map_err(|_| RestError::BadRequest)?;

    // Create new crc32 hasher to calculate the checksum
    let mut hasher = crc32fast::Hasher::new();

    // Create a new local file
    let mut file = fs::File::create(Path::new(&config.server.file_output_path).join(filename))
        .await
        .map_err::<RestError, _>(|i| i.into())?;

    let mut size: i64 = 0;
    let mut mime_type: Option<String> = None;

    // Write part
    while let Some(chunk) = part.next().await {
        let data = chunk.map_err(|_| RestError::UnknownIO)?;

        // parse filetype on first chunk
        if size == 0 {
            mime_type = infer::get(&data).map(|i| i.mime_type().to_string());
        }

        // Write to file
        file.write_all(&data).await?;

        // Update crc32 hash
        hasher.update(&data);

        size += data.len() as i64;
    }

    let crc = format!("{:x}", hasher.finalize());

    Ok((crc, size, mime_type.unwrap_or_default()))
}

/// Try to get the desired namespace. Use the precached
/// namespace if possible and desired
fn retrieve_namespace(
    upload_request: &UploadRequest,
    user: &Authenticateduser,
    db: &DbConnection,
) -> Result<Namespace, RestError> {
    let ns_name = upload_request
        .attributes
        .as_ref()
        .and_then(|attr| Some(attr.namespace.clone().to_lowercase()))
        .unwrap_or("default".to_string());

    Ok({
        if ns_name == "default" {
            user.default_ns
                .as_ref()
                .map(|i| i.clone())
                .unwrap_or(user.user.get_default_namespace(&db)?)
        } else {
            Namespace::find_by_name(&db, &ns_name, user.user.id)?
                .ok_or_else(|| RestError::NotFound)?
        }
    })
}
