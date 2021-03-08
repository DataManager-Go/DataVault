use crate::{
    config::Config,
    models::NewUser,
    response_code::{RestError, Success, SUCCESS},
    utils, DbConnection, DbPool,
};

use actix_web::web::{self, Json};
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error::DatabaseError},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    mid: Option<String>,
    username: String,
    pass: String,
}

/// Endpoint for registering new users
pub async fn ep_register(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req: web::Json<RegisterRequest>,
) -> Result<Json<Success>, RestError> {
    config
        .server
        .allow_registration
        .then(|| false)
        .ok_or(RestError::Forbidden)?;

    if req.username.is_empty() || req.pass.is_empty() {
        return Err(RestError::BadRequest);
    }

    let db = pool.get()?;

    web::block(move || register(&db, &req.username, &req.pass)).await?;

    Ok(SUCCESS)
}

/// Register a new user
pub fn register(db: &DbConnection, username: &str, password: &str) -> Result<(), RestError> {
    use crate::schema::users;

    let password = &utils::sha512(&[&username, &password]);

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
