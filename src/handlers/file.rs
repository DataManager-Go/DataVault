use super::authentication::Authenticateduser;
use crate::{
    config::Config,
    response_code::{RestError, Success, SUCCESS},
    DbPool,
};

use actix_web::web::{self, Json};
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error::DatabaseError},
};
use serde::{Deserialize, Serialize};

/// Endpoint for registering new users
pub async fn ep_list_files(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    user: Authenticateduser,
) -> Result<Json<Success>, RestError> {
    Ok(SUCCESS)
}
