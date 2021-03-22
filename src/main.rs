#![allow(clippy::new_ret_no_self)]

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

#[macro_use]
extern crate log;

#[macro_use]
extern crate diesel;
extern crate dotenv;

pub mod config;
mod db;
mod handlers;
pub mod models;
mod response_code;
mod schema;
pub mod utils;

use std::path::Path;

use actix_files::NamedFile;
use actix_web::{
    dev,
    http::{self, header::LOCATION, HeaderValue},
    middleware::{self, ErrorHandlerResponse, ErrorHandlers},
    web, App, HttpResponse, HttpServer,
};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use handlers::{attributes, namespace};

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    let config = config::Config::new().await.expect("Couldn't load config");

    // Create local filepath if net yet exists
    let local_file_path = Path::new(&config.server.file_output_path);
    if !local_file_path.exists() {
        std::fs::create_dir_all(local_file_path).expect("Coudln't create local file directory");
    }

    let ressorce_dir = config
        .server
        .html_files
        .clone()
        .unwrap_or_else(|| "html".to_string());

    // Check for html ressources
    if !Path::new(&ressorce_dir).exists() {
        panic!("Missing html files");
    }

    let db = db::connect();
    let listen_address = config.server.listen_address.clone();

    HttpServer::new(move || {
        App::new()
            // Data
            .data(config.clone())
            .data(db.clone())
            .app_data(db.clone())
            // Middlewares
            .wrap(middleware::Logger::default())
            // Static files
            .route("/index.html", web::get().to(index))
            .route("/", web::get().to(index))
            .service(actix_files::Files::new("/static", "html/static").show_files_listing())
            .default_service(actix_files::Files::new("/static", "html/static").show_files_listing())
            // Preview
            .service(
                web::resource("/preview/raw/{fileID}")
                    .to(handlers::web::raw_file_preview::ep_preview_raw),
            )
            .service(web::resource("/preview/{fileID}").to(handlers::web::preview::ep_preview))
            // API endpoints
            .service(web::resource("/user/register").to(handlers::user::ep_register))
            .service(web::resource("/user/login").to(handlers::user::ep_login))
            .service(web::resource("/user/stats").to(handlers::user::ep_stats))
            .service(web::resource("/files").to(handlers::list_file::ep_list_files))
            .service(web::resource("/download/file").to(handlers::file_action::ep_file_download))
            .service(web::resource("/file/publish").to(handlers::file_action::ep_publish_file))
            .service(web::resource("/file/{action}").to(handlers::file_action::ep_file_action))
            .service(web::resource("/attribute/{type}/get").to(attributes::ep_list_attributes))
            .service(
                web::resource("/attribute/{type}/{action}").to(attributes::ep_attribute_action),
            )
            .service(web::resource("/ping").to(handlers::ping::ep_ping))
            .service(web::resource("/namespace/create").to(namespace::ep_create_namespace))
            .service(web::resource("/namespaces").to(namespace::ep_list_namespace))
            .service(web::resource("/namespace/update").to(namespace::ep_rename_namespace))
            .service(web::resource("/upload/file").to(handlers::upload_file::ep_upload))
            .service(web::resource("/namespace/delete").to(namespace::ep_delete_namespace))
            // Other
            .default_service(web::route().to(HttpResponse::NotFound))
            .wrap(ErrorHandlers::new().handler(http::StatusCode::NOT_FOUND, redirect_home_handler))
    })
    .bind(listen_address)?
    .run()
    .await
}

/// Serve index file
async fn index() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open(Path::new("html/index.html"))?)
}

/// Redirect to home
#[allow(clippy::clippy::unnecessary_wraps)]
fn redirect_home_handler<B>(
    mut res: dev::ServiceResponse<B>,
) -> actix_web::Result<ErrorHandlerResponse<B>> {
    res.headers_mut()
        .insert(LOCATION, HeaderValue::from_static("/"));
    *res.response_mut().status_mut() = http::StatusCode::MOVED_PERMANENTLY;
    Ok(ErrorHandlerResponse::Response(res))
}
