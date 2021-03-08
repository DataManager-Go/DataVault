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
use handlers::file::ep_list_files;
use handlers::user::{ep_login, ep_register};

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
            .service(web::resource("/user/register").to(ep_register))
            .service(web::resource("/user/login").to(ep_login))
            .service(web::resource("/files").to(ep_list_files))
            .service(web::resource("/ping").to(handlers::ping::ep_ping))
            .service(
                web::resource("/namespace/create").to(handlers::namespace::ep_create_namespace),
            )
            .service(web::resource("/namespaces").to(handlers::namespace::ep_list_namespace))
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
