use super::authentication::Authenticateduser;
use crate::{
    config::Config,
    response_code::{RestError, Success, SUCCESS},
    DbPool,
};

use actix_web::web::{self, Json};

/// Endpoint for registering new users
pub async fn ep_list_files(
    _pool: web::Data<DbPool>,
    _config: web::Data<Config>,
    _user: Authenticateduser,
) -> Result<Json<Success>, RestError> {
    Ok(SUCCESS)
}
