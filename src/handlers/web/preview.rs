use actix_web::{http::header::USER_AGENT, web, HttpRequest, HttpResponse};
use humansize::{file_size_opts as options, FileSize};
use itertools::Itertools;
use lazy_static::lazy_static;

use crate::{config::Config, models::file::File, response_code::RestError, templates, DbPool};

use super::raw_file_preview;

lazy_static! {
    pub static ref DEFAULT_ACE_THEME: String = String::from("nord_dark");
    pub static ref DEFAULT_PREVIEW_FS_LIMIT: i64 = 30 * 1024; // 30kb
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PreviewType {
    Text,
    Image,
    Video,
    Archive,
    Fallback,
}

/// Endpoint for registering new users
pub async fn ep_preview(
    file_id: web::Path<String>,
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    request: HttpRequest,
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

    let preview_type = get_preview_type(&file);
    let preview_size_limit = config
        .server
        .max_preview_filesize
        .map(|i| i as i64)
        .unwrap_or(*DEFAULT_PREVIEW_FS_LIMIT);

    // return raw fie if the requesing useragent is in the raw_file_agents list
    if (file.file_size > preview_size_limit && preview_type == PreviewType::Text)
        || check_is_raw_agent(&request, &config)
        || is_raw_preview_file(&file)
    {
        return raw_file_preview::serve_file(&file, &config).await;
    }

    let host = &config.server.external_url;
    let ace_theme = config
        .preview
        .ace_theme
        .as_ref()
        .unwrap_or(&DEFAULT_ACE_THEME);

    Ok(HttpResponse::Ok().body(render!(
        templates::preview,
        host,
        &ace_theme,
        &file,
        &config
    )))
}

/// Returns the type of preview the frontend should load
pub fn get_preview_type(file: &File) -> PreviewType {
    let ftype = &file.file_type;
    let (type_, extension) = {
        if ftype.contains('/') {
            let split = ftype.split('/').collect_vec();
            (split[0], split[1])
        } else {
            (ftype.as_str(), "")
        }
    };

    match type_ {
        "image" => PreviewType::Image,
        "video" => PreviewType::Video,
        "text" => PreviewType::Text,
        _ => match extension {
            "x-tar" | "zip" => PreviewType::Archive,
            _ => PreviewType::Fallback,
        },
    }
}

/// Return true if a file previewed 'raw'
fn is_raw_preview_file(file: &File) -> bool {
    file.file_type.contains("application/pdf") || file.file_type.contains("audio")
}

/// Get a files size human readabe
pub fn file_size_humanized(file: &File) -> String {
    file.file_size.file_size(options::CONVENTIONAL).unwrap()
}

fn check_is_raw_agent(request: &HttpRequest, config: &Config) -> bool {
    if let Some(ref raw_agents) = config.server.raw_file_agents {
        if let Some(agent) = request
            .headers()
            .get(USER_AGENT)
            .map(|i| i.to_str().unwrap_or("").to_lowercase())
        {
            return !agent.is_empty() && raw_agents.iter().any(|i| agent.contains(i));
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{get_preview_type, PreviewType};
    use crate::models::file::File;

    fn get_file(mime: &str) -> File {
        File {
            file_type: mime.to_owned(),
            ..File::default()
        }
    }

    #[test]
    fn test_preview_type_img() {
        assert_eq!(get_preview_type(&get_file("image/png")), PreviewType::Image)
    }

    #[test]
    fn test_preview_type_text() {
        assert_eq!(get_preview_type(&get_file("text/plain")), PreviewType::Text)
    }

    #[test]
    fn test_preview_type_fallback_invalid() {
        assert_eq!(
            get_preview_type(&get_file("applicatio")),
            PreviewType::Fallback
        )
    }

    #[test]
    fn test_preview_type_fallback() {
        assert_eq!(
            get_preview_type(&get_file("application/pdf")),
            PreviewType::Fallback
        )
    }

    #[test]
    fn test_preview_type_video() {
        assert_eq!(get_preview_type(&get_file("video/mp4")), PreviewType::Video)
    }

    #[test]
    fn test_preview_type_fail() {
        assert_ne!(get_preview_type(&get_file("video/mp4")), PreviewType::Text)
    }
}
