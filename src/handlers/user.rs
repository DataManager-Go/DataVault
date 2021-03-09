use crate::{
    config::Config,
    models::user::User,
    response_code::{RestError, Success, SUCCESS},
    DbPool,
};

use actix_web::web::{self, Json};

use super::{requests::CredentialsRequest, response::LoginResponse};

/// Endpoint for registering new users
pub async fn ep_register(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req: web::Json<CredentialsRequest>,
) -> Result<Json<Success>, RestError> {
    // Don't allow the registration ep if disabled in config
    config
        .server
        .allow_registration
        .then(|| false)
        .ok_or(RestError::Forbidden)?;

    if req.has_empty() {
        return Err(RestError::BadRequest);
    }

    let db = pool.get()?;

    let new_user = User::new(req.username.clone(), req.password.clone());

    web::block(move || new_user.create(&db)).await?;

    Ok(SUCCESS)
}

/// Endpoint for loggin in users
pub async fn ep_login(
    pool: web::Data<DbPool>,
    req: web::Json<CredentialsRequest>,
) -> Result<Json<LoginResponse>, RestError> {
    if req.has_empty() {
        return Err(RestError::BadRequest);
    }

    let db = pool.get()?;

    let token =
        web::block(move || User::login(&db, &req.username, &req.password, &req.machine_id)).await?;

    Ok(Json(LoginResponse { token }))
}
