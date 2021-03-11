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

    // Find namespace
    let attributes = upload_request.attributes.clone();
    let db2 = pool.get()?;
    let user_clone = user.clone();
    let mut target_namespace =
        web::block(move || retrieve_namespace(&attributes, &user_clone, &db2)).await?;

    // TODO url upload

    let mut file: File = File::default();
    let mut new_file = true; // Whether to create a new file or update an existing one
    let mut replace_file = false;

    // Replace file with same name
    if upload_request.replace_equal_names {
        let db = pool.get()?;
        let name = upload_request.name.clone();
        let ns = target_namespace.id;

        let count = web::block(move || File::find_by_name_count(&db, name, ns)).await?;

        if count > 1 {
            return Err(RestError::MultipleFilesMatch);
        } else if count == 1 {
            let db2 = pool.get()?;
            let name2 = upload_request.name.clone();
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

        // Set target_namespace to file's ns
        let ns_id = file.namespace_id;
        let db = pool.get()?;
        target_namespace = web::block(move || Namespace::find_by_id(&db, ns_id)).await?;
    } else if !replace_file {
        // Create a new file
        new_file = true;

        file = upload_request.clone().into();
        file.namespace_id = target_namespace.id;
        file.user_id = user.user.id;
    }

    // TODO set groups and tags

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
    attributes: &Option<FileAttributes>,
    user: &Authenticateduser,
    db: &DbConnection,
) -> Result<Namespace, RestError> {
    let ns_name = attributes
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
