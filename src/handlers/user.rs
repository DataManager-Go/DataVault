use crate::{
    config::Config,
    models::user::User,
    response_code::{RestError, Success, SUCCESS},
    DbPool,
};

use actix_web::web::{self, Json};

use super::{
    authentication::Authenticateduser,
    requests::CredentialsRequest,
    response::{LoginResponse, StatsResponse},
};

/// Endpoint for registering new users
pub async fn ep_register(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req: web::Json<CredentialsRequest>,
) -> Result<Json<Success>, RestError> {
    let req = req.into_inner();

    // Don't allow the registration ep if disabled in config
    config
        .server
        .allow_registration
        .then(|| false)
        .ok_or(RestError::Forbidden)?;

    if req.has_empty() {
        return Err(RestError::BadRequest);
    }

    let new_user = User::new(req.username, req.password);
    let db = pool.get()?;
    web::block(move || new_user.create(&db)).await??;

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

    let token = web::block(move || User::login(&db, &req.username, &req.password, &req.machine_id))
        .await??;

    Ok(Json(LoginResponse { token }))
}

/// Endpoint for loggin in users
pub async fn ep_stats(
    pool: web::Data<DbPool>,
    user: Authenticateduser,
) -> Result<Json<StatsResponse>, RestError> {
    let db = pool.get()?;

    let res = web::block(move || -> Result<StatsResponse, RestError> {
        let total_files = user.user.total_filecount(&db)?;
        let total_filesize = user.user.total_filesize(&db)?;
        let namespaces_count = user.user.total_namespace_count(&db)?;
        let (tag_count, group_count) = user.user.total_attribute_count(&db)?;

        Ok(StatsResponse {
            files_uploaded: total_files,
            namespaces_count,
            tag_count,
            group_count,
            total_filesize,
            ..StatsResponse::default()
        })
    })
    .await??;

    Ok(Json(res))
}
