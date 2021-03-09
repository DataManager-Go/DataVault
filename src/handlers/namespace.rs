use super::{authentication::Authenticateduser, requests::NamespaceRequest, response::VecResponse};
use crate::{
    models::namespace::{self, Namespace},
    response_code::{RestError, Success, SUCCESS},
    DbPool,
};

use actix_web::web::{self, Json};

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

/// Endpoint for listing available namespaces for a user
pub async fn ep_list_namespace(
    pool: web::Data<DbPool>,
    user: Authenticateduser,
) -> Result<Json<VecResponse<String>>, RestError> {
    let db = pool.get()?;

    let ns_names = web::block(move || Namespace::list(&db, user.user))
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

/// Endpoint for renaming a namespace
pub async fn ep_rename_namespace(
    pool: web::Data<DbPool>,
    user: Authenticateduser,
    req: web::Json<NamespaceRequest>,
) -> Result<Json<Success>, RestError> {
    if req.new_name.is_none() {
        return Err(RestError::BadRequest);
    }

    let db = pool.get()?;

    web::block(move || -> Result<(), RestError> {
        let namespace = Namespace::find_by_name(&db, &req.name, user.user.id)
            .map_err::<RestError, _>(|i| i.into())?;

        if let Some(ns) = namespace {
            ns.rename(&db, req.new_name.as_ref().unwrap())
                .map_err::<RestError, _>(|i| i.into())?;
            Ok(())
        } else {
            Err(RestError::NotFound)
        }
    })
    .await?;

    Ok(SUCCESS)
}
