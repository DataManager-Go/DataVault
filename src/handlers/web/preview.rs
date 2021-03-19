use actix_web::{http::header::USER_AGENT, web, HttpRequest, HttpResponse};
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PreviewType {
    Text,
    Image,
    Viedo,
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

    // return raw fie if the requesing useragent is in the raw_file_agents list
    if check_is_raw_agent(&request, &config) {
        return super::raw_file_preview::ep_preview_raw(file_id, pool, config).await;
    }

    // Find file
    let file = web::block(move || File::get_public_file(&db, &file_id))
        .await?
        .map_err(|i| response_code::diesel_option(i, Origin::File))?;

    let host = &config.server.external_url;
    let ace_theme = config
        .preview
        .ace_theme
        .as_ref()
        .unwrap_or_else(|| &DEFAULT_ACE_THEME);

    Ok(HttpResponse::Ok().body(render!(crate::templates::preview, host, &ace_theme, &file)))
}

fn check_is_raw_agent(request: &HttpRequest, config: &Config) -> bool {
    if let Some(ref raw_agents) = config.raw_file_agents {
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
