#[macro_use]
extern crate log;

#[macro_use]
extern crate diesel;
extern crate dotenv;

pub mod config;
mod db;
mod handlers;
mod models;
mod response_code;
mod schema;
pub mod utils;

use actix_web::{middleware, web, App, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use handlers::user::{ep_login, ep_register};

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    let db = db::connect();

    let config = config::Config::new().await.expect("Couldn't load config");

    HttpServer::new(move || {
        App::new()
            .data(db.clone())
            .data(config.clone())
            .service(web::resource("/user/register").to(ep_register))
            .service(web::resource("/user/login").to(ep_login))
            .wrap(middleware::Logger::default())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
