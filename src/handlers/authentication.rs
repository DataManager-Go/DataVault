use crate::models::{login_session, namespace::Namespace};
use crate::{models::user::User, DbPool};
use actix_web::{error::ErrorInternalServerError, web::Data, Error, FromRequest, HttpRequest};
use futures::future::{err, ok, Ready};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    /// Prevent unnecessary 'default_namespace' DB calls by lazy loading them into memory
    static ref NS_CACHE: Mutex<HashMap<i32, Option<Namespace>>> = Mutex::new(HashMap::new());
}

/// Defines a struct which implements FromRequest.
/// This allows passing as requirement for a request
/// and results in a valid session being required
#[derive(Clone)]
pub struct Authenticateduser {
    pub default_ns: Option<Namespace>,
    pub user: User,
    pub token: String,
}

impl FromRequest for Authenticateduser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        // Look up session if passed token/header is valid
        if let Some(token) = get_bearer_token(req) {
            let db = req.app_data::<Data<DbPool>>().and_then(|i| i.get().ok());
            if db.is_none() {
                return err(ErrorInternalServerError("Error"));
            }
            let db = db.unwrap();

            // Find session by token
            let user = match login_session::find_session(&db, &token) {
                Ok(user) => match user {
                    Some(user) => user,
                    // Token was not found
                    None => return err(actix_web::error::ErrorUnauthorized("Not authorized")),
                },

                // An unexpected error occured
                Err(_) => return err(ErrorInternalServerError("Error")),
            };

            // Disable disabled user // **pun not intended!!!
            if user.disabled {
                return err(actix_web::error::ErrorUnauthorized("User disabled"));
            }

            let mut ns_cache = match NS_CACHE.lock() {
                Ok(cache) => cache,
                Err(_) => return err(actix_web::error::ErrorInternalServerError("Fatal error")),
            };
            let default_ns = ns_cache
                .entry(user.id)
                .or_insert_with(|| user.get_default_namespace(&db).ok());

            // Success
            return ok(Authenticateduser {
                user,
                token,
                default_ns: default_ns.clone(),
            });
        }

        err(actix_web::error::ErrorUnauthorized("Not authorized"))
    }
}

/// Get the bearer token from request headers
pub fn get_bearer_token(req: &HttpRequest) -> Option<String> {
    req.headers().get("Authorization").and_then(|i| {
        i.to_str().map(String::from).ok().and_then(|header| {
            if header.trim().contains(' ') {
                Some(header.split(' ').collect::<Vec<&str>>()[1].to_owned())
            } else {
                None
            }
        })
    })
}
