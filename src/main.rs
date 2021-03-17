#![allow(clippy::new_ret_no_self)]

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

use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    let config = config::Config::new().await.expect("Couldn't load config");
    let db = db::connect();

    HttpServer::new(move || {
        App::new()
            // Data
            .data(config.clone())
            .data(db.clone())
            .app_data(db.clone())
            // Middlewares
            .wrap(middleware::Logger::default())
            // Services
            .service(web::resource("/user/register").to(handlers::user::ep_register))
            .service(web::resource("/user/login").to(handlers::user::ep_login))
            .service(web::resource("/files").to(handlers::list_file::ep_list_files))
            .service(web::resource("/download/file").to(handlers::file_action::ep_file_download))
            .service(web::resource("/file/publish").to(handlers::file_action::ep_publish_file))
            .service(web::resource("/file/{action}").to(handlers::file_action::ep_file_action))
            .service(
                web::resource("/attribute/{type}/get").to(handlers::attributes::ep_list_attributes),
            )
            .service(
                web::resource("/attribute/{type}/{action}")
                    .to(handlers::attributes::ep_attribute_action),
            )
            .service(web::resource("/ping").to(handlers::ping::ep_ping))
            .service(
                web::resource("/namespace/create").to(handlers::namespace::ep_create_namespace),
            )
            .service(web::resource("/namespaces").to(handlers::namespace::ep_list_namespace))
            .service(
                web::resource("/namespace/update").to(handlers::namespace::ep_rename_namespace),
            )
            .service(web::resource("/upload/file").to(handlers::upload_file::ep_upload))
            .service(
                web::resource("/namespace/delete").to(handlers::namespace::ep_delete_namespace),
            )
            // Other
            .default_service(web::route().to(HttpResponse::MethodNotAllowed))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
