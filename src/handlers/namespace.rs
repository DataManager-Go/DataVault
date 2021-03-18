use super::{authentication::Authenticateduser, requests::NamespaceRequest, response::VecResponse};
use crate::{
    config::Config,
    models::namespace::{self, Namespace},
    response_code::{Origin, RestError, Success, SUCCESS},
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

    // Don't allow creating 'default' namespaces
    if Namespace::is_default_name(&req.name) {
        return Err(RestError::IllegalOperation);
    }

    web::block(move || -> Result<(), RestError> {
        namespace::CreateNamespace::new(&req.name, user.user.id).create(&pool.get()?)
    })
    .await??;

    Ok(SUCCESS)
}

/// Endpoint for listing available namespaces for a user
pub async fn ep_list_namespace(
    pool: web::Data<DbPool>,
    user: Authenticateduser,
) -> Result<Json<VecResponse<String>>, RestError> {
    let db = pool.get()?;

    let ns_names = web::block(move || Namespace::list(&db, &user.user))
        .await??
        .into_iter()
        .map(|i| i.name)
        .collect::<Vec<String>>();

    Ok(Json(VecResponse { slice: ns_names }))
}

/// Endpoint for deleting a namespace
pub async fn ep_delete_namespace(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    user: Authenticateduser,
    req: web::Json<NamespaceRequest>,
) -> Result<Json<Success>, RestError> {
    let db = pool.get()?;

    // Don't allow deleting 'default' namespaces
    if Namespace::is_default_name(&req.name) {
        return Err(RestError::IllegalOperation);
    }

    web::block(move || -> Result<(), RestError> {
        if let Some(ns) = Namespace::find_by_name(&db, &req.name, user.user.id)? {
            ns.delete(&db, &config)?;
            Ok(())
        } else {
            Err(RestError::DNotFound(Origin::Namespace))
        }
    })
    .await??;

    Ok(SUCCESS)
}

/// Endpoint for renaming a namespace
pub async fn ep_rename_namespace(
    pool: web::Data<DbPool>,
    user: Authenticateduser,
    req: web::Json<NamespaceRequest>,
) -> Result<Json<Success>, RestError> {
    let new_name = match &req.new_name {
        Some(name) => name.clone(),
        None => return Err(RestError::BadRequest),
    };

    // Don't allow modifying 'default' namespaces
    if Namespace::is_default_name(&req.name) || Namespace::is_default_name(&new_name) {
        return Err(RestError::IllegalOperation);
    }

    if req.name == new_name {
        return Err(RestError::BadRequest);
    }

    let db = pool.get()?;

    web::block(move || -> Result<(), RestError> {
        if let Some(ns) = Namespace::find_by_name(&db, &req.name, user.user.id)? {
            ns.rename(&db, new_name.as_ref())?;
            Ok(())
        } else {
            Err(RestError::DNotFound(Origin::Namespace))
        }
    })
    .await??;

    Ok(SUCCESS)
}
