use super::{
    authentication::Authenticateduser,
    requests::upload_request::{FileAttributes, UploadRequest},
    response::UploadResponse,
};
use crate::{
    config::Config,
    models::file::{File, NewFile},
    models::namespace::Namespace,
    response_code::RestError,
    DbConnection, DbPool,
};
use actix_multipart::Multipart;

use actix_web::web::{self, Json};
use async_std::{fs, io::prelude::*, io::BufWriter, path::Path};
use futures::StreamExt;
use lazy_static::__Deref;

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
    let request_cloned = upload_request.clone();
    let user_cloned = user.clone();
    let (mut file, namespace) =
        web::block(move || select_file(&request_cloned, &db, user_cloned)).await??;

    // TODO set groups and tags

    let (crc, size, mime_type) =
        multipart_to_file(payload, config.deref(), &file.local_name).await?;

    file.checksum = crc.clone();
    file.file_size = size;
    file.file_type = mime_type;

    debug!("{:?}", file);

    let id = if file.id == 0 {
        let new_file: NewFile = file.clone().into();
        new_file.create(&pool.get()?)?
    } else {
        file.save(&pool.get()?)?;
        file.id
    };

    Ok(Json(UploadResponse {
        file_size: size,
        checksum: crc,
        namespace: namespace.name,
        file_id: id,
        file_name: file.name,
        public_file_name: None,
    }))
}

/// Create or find a file object, on which the upload function
/// should be applied. The create_new_file return indicates
/// whether the file should be updated or inserted
fn select_file(
    upload_request: &UploadRequest,
    db: &DbConnection,
    user: Authenticateduser,
) -> Result<(File, Namespace), RestError> {
    let mut target_namespace = retrieve_namespace(&upload_request.attributes, &user, db)?;

    let mut file: File = File::default();
    let mut replace_file = false;

    // Find by name
    if upload_request.replace_equal_names {
        match File::find_by_name_count(db, &upload_request.name, target_namespace.id)? {
            0 => (),
            1 => {
                file = File::find_by_name(db, &upload_request.name, target_namespace.id)?;
                replace_file = true;
            }
            _ => return Err(RestError::MultipleFilesMatch),
        }
    }

    if let Some(id) = upload_request.replace_file_by_id {
        // Replace file by id
        // TODO merge into one request

        file = File::find_by_id(db, id, user.user.id)?;

        // Set target_namespace to file's ns
        target_namespace = Namespace::find_by_id(db, file.namespace_id)?;
    } else if !replace_file {
        // Create a new file
        file = upload_request.clone().into();
        file.namespace_id = target_namespace.id;
        file.user_id = user.user.id;
    }

    Ok((file, target_namespace))
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

    let mut file_writer = BufWriter::new(&file);

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
        file_writer.write_all(&data).await?;

        // Update crc32 hash
        hasher.update(&data);

        size += data.len() as i64;
    }

    file_writer.flush().await?;
    file.sync_data().await?;

    let crc = format!("{:x}", hasher.finalize());

    Ok((crc, size, mime_type.unwrap_or_default()))
}

/// Try to get the desired namespace. Use the precached
/// namespace if possible and desired
fn retrieve_namespace(
    attributes: &Option<FileAttributes>,
    user: &Authenticateduser,
    db: &DbConnection,
) -> Result<Namespace, RestError> {
    let ns_name = attributes
        .as_ref()
        .map(|i| i.namespace.clone())
        .unwrap_or_else(|| "default".to_string());

    Ok({
        if Namespace::is_default_name(&ns_name) {
            user.default_ns
                .as_ref()
                .cloned()
                .unwrap_or(user.user.get_default_namespace(&db)?)
        } else {
            Namespace::find_by_name(&db, &ns_name, user.user.id)?.ok_or(RestError::NotFound)?
        }
    })
}
