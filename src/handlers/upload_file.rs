use std::{cmp, path::Path};

use super::{
    authentication::Authenticateduser, requests::upload_request::UploadRequest,
    response::UploadResponse, utils::retrieve_namespace,
};
use crate::{
    config::Config,
    models::{
        attribute,
        file::{File, NewFile},
        namespace::Namespace,
    },
    response_code::RestError,
    utils, DbConnection, DbPool,
};

use actix_web::{
    dev::Decompress,
    web::{self, Json, Payload},
    HttpRequest,
};
use async_std::{fs, io::prelude::*};
use attribute::{
    AttributeType::{Group, Tag},
    NewAttribute,
};
use crc32fast::Hasher;
use futures::StreamExt;
use itertools::Itertools;
use lazy_static::__Deref;

/// Endpoint for uploading new files
pub async fn ep_upload(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    user: Authenticateduser,
    upload_request: UploadRequest,
    payload: Payload,
    request: HttpRequest,
) -> Result<Json<UploadResponse>, RestError> {
    upload_request.validate(&user)?;

    // Pick correct file
    let db = pool.get()?;
    let request_cloned = upload_request.clone();
    let user_cloned = user.clone();
    let (file, namespace) =
        web::block(move || select_file(&request_cloned, &db, user_cloned)).await??;

    let upload = UploadHanler {
        payload,
        config: config.clone(),
        file: file.clone(),
        request,
        pool,
        upload_request,
        user,
        namespace,
    };

    // Continue in out-sourced function
    // to delete the local file on an error
    let result = upload.handle().await;
    if let Err(err) = result {
        // Delete local file on fail
        fs::remove_file(Path::new(&config.server.file_output_path).join(&file.local_name))
            .await
            .ok();

        Err(err)
    } else {
        Ok(result.unwrap())
    }
}

// Prevent too many parameters
struct UploadHanler {
    payload: Payload,
    config: web::Data<Config>,
    file: File,
    request: HttpRequest,
    pool: web::Data<r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>>,
    upload_request: UploadRequest,
    user: Authenticateduser,
    namespace: Namespace,
}

impl UploadHanler {
    /// Moved to extra function to catch error easily
    /// and delete the local file on an error
    async fn handle(mut self) -> Result<Json<UploadResponse>, RestError> {
        let (crc, size, mime_type) = save_to_file(
            self.payload,
            self.config.deref(),
            &self.file.local_name,
            self.request,
        )
        .await?;
        self.file.checksum = crc.clone();
        self.file.file_size = size;
        self.file.file_type = mime_type;
        let db = self.pool.get()?;

        self.file.id = {
            if self.file.id == 0 {
                let new_file: NewFile = self.file.clone().into();
                new_file.create(&db)?.id
            } else {
                self.file.save(&db)?;
                self.file.id
            }
        };

        handle_attributes(
            &db,
            &self.upload_request,
            &self.file,
            &self.user,
            &self.namespace,
        )?;

        Ok(Json(UploadResponse {
            file_size: size,
            checksum: crc,
            namespace: self.namespace.name,
            file_id: self.file.id,
            file_name: self.file.name,
            public_file_name: self.file.public_filename,
        }))
    }
}

/// Create or find a file object, on which the upload function
/// should be applied. The create_new_file return indicates
/// whether the file should be updated or inserted
fn select_file(
    upload_request: &UploadRequest,
    db: &DbConnection,
    user: Authenticateduser,
) -> Result<(File, Namespace), RestError> {
    let mut target_namespace = retrieve_namespace(db, &upload_request.attributes.as_ref(), &user)?;

    let mut file: File = File::default();
    let mut replace_file = false;

    // Find by name
    if upload_request.replace_equal_names {
        match File::find_by_name_count(db, &upload_request.name, target_namespace.id)? {
            0 => (), // No file found, continue as usual and create a new one
            1 => {
                // One file found, replace this one
                file = File::find_by_name(db, &upload_request.name, target_namespace.id)?;
                replace_file = true;
            }
            _ => return Err(RestError::MultipleFilesMatch), // More than one file found, prevent overwriting the first one
        }
    }

    if let Some(id) = upload_request.replace_file_by_id {
        // Replace file by id
        file = File::find_by_id(db, id, user.user.id)?;
        target_namespace = file.namespace(db)?;
    } else if !replace_file {
        // Create a new file
        file = upload_request.clone().into();
        file.namespace_id = target_namespace.id;
        file.user_id = user.user.id;

        // Make public
        // TODO check for collisions first
        if upload_request.public.unwrap_or(false) {
            file.public_filename = Some(
                upload_request
                    .public_name
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| utils::random_string(25)),
            );
        }
    }

    Ok((file, target_namespace))
}

/// Creates and adds requested attributes
/// to the uploaded file
fn handle_attributes(
    db: &DbConnection,
    upload_request: &UploadRequest,
    file: &File,
    user: &Authenticateduser,
    namespace: &Namespace,
) -> Result<(), RestError> {
    if let Some(ref attributes) = upload_request.attributes {
        // Get and create tags
        let tags = if let Some(ref tags) = attributes.tags {
            NewAttribute::find_and_create(&db, &tags, Tag, user.user.id, namespace.id)?
        } else {
            vec![]
        };

        // Get and create groups
        let groups = if let Some(ref groups) = attributes.groups {
            NewAttribute::find_and_create(&db, &groups, Group, user.user.id, namespace.id)?
        } else {
            vec![]
        };

        // concat vectors and add to file
        file.add_attributes(&db, [tags, groups].concat())?;
    }

    Ok(())
}

/// Write a multipart to a given file. Returns
/// (crc32, size, mimeType)
pub async fn save_to_file(
    body: Payload,
    config: &Config,
    filename: &str,
    request: HttpRequest,
) -> Result<(String, i64, String), RestError> {
    // Create new crc32 hasher to calculate the checksum
    let mut hasher = crc32fast::Hasher::new();

    // Create a new local file
    let mut file =
        fs::File::create(Path::new(&config.server.file_output_path).join(filename)).await?;

    let mut size: i64 = 0;
    let mut mime_type: Option<String> = None;

    // Use header to determine whether the file should be decompressed
    let mut stream = Decompress::from_headers(body, request.headers());

    let mut buf = UploadBuffer::new(8);

    let mut contains_binary = false;

    // Write part
    while let Some(chunk) = stream.next().await {
        let data = chunk.map_err(|_| RestError::UnknownIO)?;

        if data.is_empty() {
            continue;
        }

        // detect filetype on first chunk
        if size == 0 {
            mime_type = infer::get(&data).map(|i| i.mime_type().to_string());
        }

        let amount = cmp::min(buf.size(), data.len());
        // Write last len(amout) bytes into the buffer
        let dropped = buf.push(&get_last_n(&data, amount));
        // Write dropped bytes into file+hasher
        write(
            &mut file,
            &mut hasher,
            &dropped,
            &mut contains_binary,
            &mime_type,
        )
        .await?;

        // Get bytes without those which were written into buffer
        let data = without_last_n(&data, amount);
        write(
            &mut file,
            &mut hasher,
            &data,
            &mut contains_binary,
            &mime_type,
        )
        .await?;

        size += (data.len() + dropped.len()) as i64;
    }

    file.flush().await?;
    file.sync_all().await?;

    if !contains_binary && mime_type.is_none() {
        mime_type = Some(String::from("text/plain"));
    }

    let crc = format!("{:08x}", hasher.finalize()).to_lowercase();
    let crc_rec = String::from_utf8(buf.get())
        .map_err(|_| RestError::PartialContent)?
        .to_lowercase();

    if crc != crc_rec {
        return Err(RestError::PartialContent);
    }

    Ok((crc, size, mime_type.unwrap_or_default()))
}

/// Write to file and hasher at the same time
async fn write(
    file: &mut fs::File,
    hasher: &mut Hasher,
    data: &[u8],
    contains_binary: &mut bool,
    mime_type: &Option<String>,
) -> Result<(), RestError> {
    file.write_all(&data).await?;
    hasher.update(&data);

    if mime_type.is_none() && !*contains_binary && std::str::from_utf8(&data).is_err() {
        *contains_binary = true;
    }

    Ok(())
}

/// Gets the last n bytes of data
fn get_last_n(data: &[u8], n: usize) -> Vec<u8> {
    data.iter().rev().take(n).rev().copied().collect_vec()
}

/// Get data without last n bytes
fn without_last_n(data: &[u8], n: usize) -> Vec<u8> {
    let take = data.len() - cmp::min(n, data.len());
    data.iter().take(take).copied().collect()
}

/// A buffer to keep track of the
/// last `size` elements in a chunked stream
#[derive(Debug, Clone)]
pub struct UploadBuffer {
    size: usize,
    buff: Vec<u8>,
    len: usize,
}

impl UploadBuffer {
    /// Create a new buffer
    pub fn new(size: usize) -> Self {
        UploadBuffer {
            size,
            buff: vec![0; size],
            len: 0,
        }
    }

    /// Push 'bytes' into the buffer and pop overflowing items
    pub fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        let will_be_popped = {
            let left_to_fill = self.size - self.len();
            if bytes.len() > left_to_fill {
                self.buff
                    .iter()
                    .rev()
                    // ensure to only pop actually overflowing items
                    .skip(left_to_fill)
                    .take(bytes.len() - left_to_fill)
                    .copied()
                    .collect_vec()
            } else {
                vec![]
            }
        };

        // Add bytes.len() to curret len, but don't go over self.size
        self.len = cmp::min(self.len + bytes.len(), self.size);

        // Push items
        for byte in bytes.iter().copied() {
            self.buff.insert(0, byte)
        }

        // Shorten buffer
        self.buff.truncate(self.size);

        will_be_popped
    }

    /// Set the buffers holding value. Never access the
    /// inner vector manually, since the buffer keeps track
    /// of the inner items in several ways
    pub fn set(&mut self, bytes: &[u8]) {
        for byte in bytes.iter().copied() {
            self.push(&[byte]);
        }
    }

    /// Get the curret buffer value
    pub fn get(&self) -> Vec<u8> {
        self.buff
            .iter()
            .copied()
            // Don't return more than self.len!
            // Required since len < size is valid
            .take(self.len())
            .rev()
            .collect_vec()
    }

    /// Return the length of the buffer
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return the size of the buffer
    pub fn size(&self) -> usize {
        self.size
    }
}
