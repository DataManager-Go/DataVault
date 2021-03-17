use actix_web::web;

use super::{authentication::Authenticateduser, requests::upload_request::FileAttributes};
use crate::{models::namespace::Namespace, response_code::RestError, DbConnection};

pub async fn retrieve_namespace_by_name_async(
    db: DbConnection,
    namespace: String,
    user: Authenticateduser,
) -> Result<Namespace, RestError> {
    web::block(move || retrieve_namespace_by_name(&db, &namespace, &user)).await?
}

/// find a namespace by its name
pub fn retrieve_namespace_by_name(
    db: &DbConnection,
    namespace: &str,
    user: &Authenticateduser,
) -> Result<Namespace, RestError> {
    retrieve_namespace(
        db,
        &Some(&FileAttributes {
            namespace: namespace.to_owned(),
            groups: None,
            tags: None,
        }),
        user,
    )
}

/// Try to get the desired namespace. Use the precached
/// namespace if possible and desired
pub fn retrieve_namespace(
    db: &DbConnection,
    attributes: &Option<&FileAttributes>,
    user: &Authenticateduser,
) -> Result<Namespace, RestError> {
    let ns_name = attributes
        .map(|i| i.namespace.clone())
        .unwrap_or_else(|| "default".to_string());

    Ok({
        if Namespace::is_default_name(&ns_name) {
            user.default_ns
                .as_ref()
                .cloned()
                .unwrap_or(user.user.get_default_namespace(&db)?)
        } else {
            Namespace::find_by_name(&db, &ns_name, user.user.id)?.ok_or(RestError::NotFound)?
        }
    })
}
