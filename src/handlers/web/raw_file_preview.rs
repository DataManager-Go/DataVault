use std::{fs, path::Path};

use actix_web::{http::header::CONTENT_LENGTH, web, HttpResponse};

use crate::{config::Config, models::file::File, response_code::RestError, DbPool};

use super::super::chunked::ChunkedReadFile;

/// Endpoint for registering new users
pub async fn ep_preview_raw(
    file_id: web::Path<String>,
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
) -> Result<HttpResponse, RestError> {
    let db = pool.get()?;

    // Find file
    let file = match web::block(move || File::get_public_file(&db, &file_id)).await? {
        Ok(o) => o,
        Err(err) => match err {
            diesel::result::Error::NotFound => return Ok(crate::to_home()),
            _ => return Err(err.into()),
        },
    };

    serve_file(&file, &config).await
}

/// Serves the content of the file
pub async fn serve_file(file: &File, config: &Config) -> Result<HttpResponse, RestError> {
    if !file.is_public {
        return Ok(crate::to_home());
    }

    // build response
    let mut response = HttpResponse::Ok();
    response.insert_header((CONTENT_LENGTH, file.file_size));

    if !file.file_type.is_empty() {
        response.insert_header(("Content-Type", format!("{};charset=UTF-8", file.file_type)));
    }

    let f = fs::File::open(Path::new(&config.server.file_output_path).join(&file.local_name))?;
    let reader = ChunkedReadFile::new(f.metadata()?.len(), 0, f);

    Ok(response.streaming(reader))
}
