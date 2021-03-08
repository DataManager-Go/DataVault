use super::{
    authentication::Authenticateduser,
    response::{StringResponse, VecResponse},
};
use crate::{
    models::namespace::{self, Namespace},
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
) -> Result<Json<StringResponse>, RestError> {
    if req.name.is_empty() {
        return Err(RestError::BadRequest);
    }

    let namespace_name = web::block(move || -> Result<String, RestError> {
        namespace::CreateNamespace::new(&req.name, user.user.id)
            .create(&pool.get()?)
            .map_err::<RestError, _>(|err| err.into())?;

        Ok(format!("{}_{}", user.user.username, req.name))
    })
    .await?;

    Ok(Json(StringResponse {
        content: namespace_name,
    }))
}

/// Endpoint for listing available namespaces for a user
pub async fn ep_list_namespace(
    pool: web::Data<DbPool>,
    user: Authenticateduser,
) -> Result<Json<VecResponse<String>>, RestError> {
    let db = pool.get()?;

    let ns_names = web::block(move || Namespace::list(&db, user.user.id))
        .await?
        .into_iter()
        .map(|i| i.name)
        .collect::<Vec<String>>();

    Ok(Json(VecResponse { slice: ns_names }))
}

/// Endpoint for deleting a namespace
pub async fn ep_delete_namespace(
    pool: web::Data<DbPool>,
    user: Authenticateduser,
    req: web::Json<NamespaceRequest>,
) -> Result<Json<Success>, RestError> {
    let db = pool.get()?;

    web::block(move || -> Result<(), RestError> {
        let namespace = Namespace::find_by_name(&db, &req.name, user.user.id)
            .map_err::<RestError, _>(|i| i.into())?;
        if let Some(ns) = namespace {
            ns.delete(&db).map_err::<RestError, _>(|i| i.into())?;
            Ok(())
        } else {
            Err(RestError::NotFound)
        }
    })
    .await?;

    Ok(SUCCESS)
}
