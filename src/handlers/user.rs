use crate::{http_errors, DbConnection};
use crate::{models::NewUser, DbPool};
use actix_web::{web, HttpResponse};
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error::DatabaseError},
};
use http_errors::RestError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

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

    register(&db, &req.username, &req.pass)?;

    Ok(HttpResponse::Ok().body(""))
}

pub fn register(db: &DbConnection, username: &str, password: &str) -> Result<(), RestError> {
    use crate::schema::users;

    let mut hasher = Sha512::new();
    hasher.update(username);
    hasher.update(password);
    let password = &format!("{:x}", hasher.finalize());

    if let Err(err) = diesel::insert_into(users::table)
        .values(&NewUser { username, password })
        .execute(db)
    {
        return Err(match err {
            DatabaseError(DatabaseErrorKind::UniqueViolation, _) => RestError::UserExists,
            _ => RestError::Unknown,
        });
    }

    Ok(())
}
