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
    DbConnection, DbPool,
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

    // Continue in out-sourced function
    // to delete the local file on an error
    let result = handle_upload(
        payload,
        config,
        file,
        request,
        pool,
        upload_request,
        user,
        namespace,
    )
    .await;
    if let Err(err) = result {
        // DELETE FILE HERE
        println!("should delete local file: {:?}", err);
        Err(err)
    } else {
        Ok(result.unwrap())
    }
}

/// Moved to extra function to catch error easily
/// and delete the local file on an error
async fn handle_upload(
    payload: Payload,
    config: web::Data<Config>,
    mut file: File,
    request: HttpRequest,
    pool: web::Data<r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>>,
    upload_request: UploadRequest,
    user: Authenticateduser,
    namespace: Namespace,
) -> Result<Json<UploadResponse>, RestError> {
    let (crc, size, mime_type) =
        save_to_file(payload, config.deref(), &file.local_name, request).await?;
    file.checksum = crc.clone();
    file.file_size = size;
    file.file_type = mime_type;
    let db = pool.get()?;

    file.id = {
        if file.id == 0 {
            let new_file: NewFile = file.clone().into();
            new_file.create(&db)?.id
        } else {
            file.save(&db)?;
            file.id
        }
    };

    handle_attributes(&db, &upload_request, &file, &user, &namespace)?;

    Ok(Json(UploadResponse {
        file_size: size,
        checksum: crc,
        namespace: namespace.name,
        file_id: file.id,
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
        write(&mut file, &mut hasher, &dropped).await?;

        // Get bytes without those which were written into buffer
        let data = without_last_n(&data, amount);
        write(&mut file, &mut hasher, &data).await?;

        size += (data.len() + dropped.len()) as i64;
    }

    file.flush().await?;
    file.sync_all().await?;

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
async fn write(file: &mut fs::File, hasher: &mut Hasher, data: &[u8]) -> Result<(), RestError> {
    file.write_all(&data).await?;
    hasher.update(&data);
    Ok(())
}

/// Gets the last n bytes of data
fn get_last_n(data: &[u8], n: usize) -> Vec<u8> {
    data.iter().rev().take(n).rev().map(|i| *i).collect_vec()
}

/// Get data without last n bytes
fn without_last_n(data: &[u8], n: usize) -> Vec<u8> {
    let take = data.len() - cmp::min(n, data.len());
    data.iter().take(take).map(|i| *i).collect()
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
        let mut buff = Vec::with_capacity(size);
        for _ in 0..size {
            buff.push(0);
        }

        UploadBuffer { size, buff, len: 0 }
    }

    /// Push 'bytes' into the buffer and pop overflowing items
    pub fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        let bytes = bytes.iter().rev().map(|i| *i).collect_vec();

        let will_be_popped = {
            let left_to_fill = self.size - self.len();
            if bytes.len() > left_to_fill {
                self.buff
                    .iter()
                    .rev()
                    // ensure to only pop actually overflowing items
                    .skip(left_to_fill)
                    .take(bytes.len() - left_to_fill)
                    // Put in right order
                    .rev()
                    .map(|i| *i)
                    .collect_vec()
            } else {
                vec![]
            }
        };
        // Add bytes.len() to curret len, but don't go over self.size
        self.len = cmp::min(self.len + bytes.len(), self.size);

        // Push items
        for byte in bytes.iter().map(|i| *i).rev() {
            self.buff.insert(0, byte)
        }

        // Shorten buffer
        self.buff.truncate(self.size);

        will_be_popped.iter().rev().map(|i| *i).collect_vec()
    }

    /// Set the buffers holding value. Never access the
    /// inner vector manually, since the buffer keeps track
    /// of the inner items in several ways
    pub fn set(&mut self, bytes: &[u8]) {
        for byte in bytes.iter().map(|i| *i).rev() {
            self.push(&[byte]);
        }
    }

    /// Get the curret buffer value
    pub fn get(&self) -> Vec<u8> {
        self.buff
            .iter()
            .map(|i| *i)
            // Don't return more than self.len
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
