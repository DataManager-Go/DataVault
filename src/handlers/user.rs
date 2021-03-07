use crate::{http_errors, DbConnection};
use crate::{models::NewUser, DbPool};
use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use http_errors::RestError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    mid: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pass: String,
}

pub async fn ep_register(
    pool: web::Data<DbPool>,
    req: web::Json<RegisterRequest>,
) -> Result<HttpResponse, http_errors::RestError> {
    // TODO add registration protection

    if req.username.is_empty() || req.pass.is_empty() {
        return Err(RestError::BadRequest);
    }

    let db = pool.get().unwrap();

    register(&db, &req.username, &req.pass);

    Ok(HttpResponse::Ok().body(""))
}

pub fn register(db: &DbConnection, username: &str, password: &str) {
    use crate::schema::users;

    let user = NewUser { username, password };

    diesel::insert_into(users::table)
        .values(&user)
        .execute(db)
        .unwrap();
}
