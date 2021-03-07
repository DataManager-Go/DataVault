use std::env;

use diesel::{r2d2::ConnectionManager, PgConnection};
use dotenv::dotenv;
use r2d2::Pool;

pub fn connect() -> Pool<ConnectionManager<PgConnection>> {
    dotenv().ok();

    let connspec = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(connspec);

    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}
