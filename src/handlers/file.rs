use super::{
    authentication::Authenticateduser, requests::upload_request::UploadRequest,
    response::UploadResponse,
};
use crate::{
    config::Config,
    models::file::{self, File, NewFile},
    models::namespace::Namespace,
    response_code::{RestError, Success, SUCCESS},
    utils, DbConnection, DbPool,
};
use actix_multipart::Multipart;

use actix_web::web::{self, Json};
use async_std::{fs, io::prelude::*, io::BufWriter, path::Path};
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
) -> Result<Json<UploadResponse>, RestError> {
    upload_request.validate(&user)?;

    let db = pool.get()?;

    // Find namespace
    // TODO don't block main thread
    let target_namespace = retrieve_namespace(&upload_request, &user, &db)?;

    // TODO url upload

    let mut file: File = File::default();
    let mut new_file = true; // Whether to save or update
    let mut replace_file = false;

    // Replace file with same name
    if upload_request.replace_equal_names {
        let db = pool.get()?;
        let db2 = pool.get()?;
        let ns = target_namespace.id;
        let name = upload_request.name.clone();
        let name2 = upload_request.name.clone();

        let count = web::block(move || File::find_by_name_count(&db, name, ns)).await?;

        if count == 0 {
            new_file = true;
            replace_file = false;
        } else if count > 1 {
            return Err(RestError::MultipleFilesMatch);
        } else {
            file = web::block(move || File::find_by_name(&db2, name2, ns)).await?;
            replace_file = true;
            new_file = false;
        }
    }

    if let Some(id) = upload_request.replace_file_by_id {
        // Replace file by id
        new_file = false;
        let db = pool.get()?;
        file = web::block(move || File::find_by_id(id, &db)).await?;
    } else if !replace_file {
        // Create a new file
        new_file = true;
        file = File {
            // Select random string as name if not provided
            name: if upload_request.name.is_empty() {
                utils::random_string(20)
            } else {
                upload_request.name.clone()
            },
            user_id: user.user.id,
            encryption: upload_request.encryption.unwrap_or(0) as i32,
            namespace_id: target_namespace.id,
            is_public: upload_request.public.unwrap_or(false),
            public_filename: upload_request.public_name,
            local_name: utils::random_string(30),
            ..file::File::default()
        };
    }

    let (crc, size, mime_type) =
        multipart_to_file(payload, config.deref(), &file.local_name).await?;

    file.checksum = crc.clone();
    file.file_size = size;
    file.file_type = mime_type;

    debug!("{:#?}", file);

    let id = if new_file {
        let new_file: NewFile = file.into();
        new_file.create(&db)?
    } else {
        file.save(&db)?;
        file.id
    };

    Ok(Json(UploadResponse {
        file_size: size,
        checksum: crc,
        namespace: target_namespace.name,
        file_id: id,
        file_name: upload_request.name,
        public_file_name: None,
    }))
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
        .ok_or(RestError::BadRequest)?
        .map_err(|_| RestError::BadRequest)?;

    // Create new crc32 hasher to calculate the checksum
    let mut hasher = crc32fast::Hasher::new();

    // Create a new local file
    let file = fs::File::create(Path::new(&config.server.file_output_path).join(filename))
        .await
        .map_err::<RestError, _>(|i| i.into())?;

    let mut file = BufWriter::new(file);

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

    file.flush().await?;

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
        .map(|i| i.namespace.clone())
        .unwrap_or_else(|| "default".to_string());

    Ok({
        if ns_name == "default" {
            user.default_ns
                .as_ref()
                .cloned()
                .unwrap_or(user.user.get_default_namespace(&db)?)
        } else {
            Namespace::find_by_name(&db, &ns_name, user.user.id)?.ok_or(RestError::NotFound)?
        }
    })
}
