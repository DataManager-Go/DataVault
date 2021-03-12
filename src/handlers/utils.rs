use super::{authentication::Authenticateduser, requests::upload_request::FileAttributes};
use crate::{models::namespace::Namespace, response_code::RestError, DbConnection};

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
