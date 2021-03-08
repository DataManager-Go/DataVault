use super::authentication::Authenticateduser;
use crate::{
    models::namespace,
    response_code::{RestError, Success, SUCCESS},
    DbPool,
};

use actix_web::web::{self, Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct NamespaceRequest {
    #[serde(rename = "ns")]
    name: String,
}

/// Endpoint for registering new users
pub async fn ep_create_namespace(
    pool: web::Data<DbPool>,
    user: Authenticateduser,
    req: web::Json<NamespaceRequest>,
) -> Result<Json<Success>, RestError> {
    if req.name.is_empty() {
        return Err(RestError::BadRequest);
    }

    web::block(move || -> Result<(), RestError> {
        namespace::CreateNamespace::new(&req.name, user.user.id)
            .create(&pool.get()?)
            .map_err(|err| err.into())
    })
    .await?;

    Ok(SUCCESS)
}
