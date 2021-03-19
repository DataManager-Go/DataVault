use actix_web::{web, HttpRequest, HttpResponse};
use lazy_static::lazy_static;
use response_code::Origin;

use crate::{
    config::Config,
    models::file::File,
    response_code::{self, RestError},
    DbPool,
};

lazy_static! {
    pub static ref DEFAULT_ACE_THEME: String = String::from("nord_dark");
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
    let file = web::block(move || File::get_public_file(&db, &file_id))
        .await?
        .map_err(|i| response_code::diesel_option(i, Origin::File))?;

    let scheme = request.uri().scheme_str().unwrap_or("http");
    let host = &config.server.external_url;
    let ace_theme = config
        .preview
        .ace_theme
        .as_ref()
        .unwrap_or_else(|| &DEFAULT_ACE_THEME);

    Ok(HttpResponse::Ok().body(render!(
        crate::templates::preview,
        scheme,
        host,
        &ace_theme,
        &file
    )))
}
