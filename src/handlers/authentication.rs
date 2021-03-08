use super::session;
use crate::{models::User, DbPool};
use actix_web::{web::Data, Error, FromRequest, HttpRequest};
use futures::future::{err, ok, Ready};

pub struct Authenticateduser {
    user: User,
    token: String,
}

impl FromRequest for Authenticateduser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(header) = auth_header.to_str().map(|i| String::from(i)) {
                if header.trim().contains(" ") {
                    let token = header.split(" ").collect::<Vec<&str>>()[1];

                    let db = req.app_data::<Data<DbPool>>().unwrap().get().unwrap();

                    let user = match session::find_session(&db, token) {
                        Ok(user) => match user {
                            Some(user) => user,
                            None => {
                                return err(actix_web::error::ErrorUnauthorized("Not authorized"))
                            }
                        },
                        Err(_) => return err(actix_web::error::ErrorInternalServerError("Error")),
                    };

                    return ok(Authenticateduser {
                        user,
                        token: token.to_owned(),
                    });
                }
            }
        }

        err(actix_web::error::ErrorUnauthorized("Not authorized"))
    }
}
