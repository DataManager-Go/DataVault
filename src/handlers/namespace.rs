use super::{authentication::Authenticateduser, ping::StringResponse};
use crate::{models::namespace, response_code::RestError, DbPool};

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
) -> Result<Json<StringResponse>, RestError> {
    if req.name.is_empty() {
        return Err(RestError::BadRequest);
    }

    let content = web::block(move || -> Result<String, RestError> {
        namespace::CreateNamespace::new(&req.name, user.user.id)
            .create(&pool.get()?)
            .map_err::<RestError, _>(|err| err.into())?;

        Ok(format!("{}_{}", user.user.username, req.name))
    })
    .await?;

    Ok(Json(StringResponse { content }))
}
