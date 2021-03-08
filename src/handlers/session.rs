use crate::{models::User, schema::users, DbConnection};
use diesel::{prelude::*, result::Error};

/// Check whether a session exists and retrieve the user
pub fn find_session(db: &DbConnection, q_token: &str) -> Result<Option<User>, Error> {
    use crate::schema::login_sessions::dsl::*;

    // Join login_sessions with users
    let user: User = match login_sessions
        .inner_join(users::table)
        .filter(token.eq(q_token))
        // Select user only
        .select((users::id, users::username, users::password, users::disabled))
        .first(db)
    {
        Ok(user) => user,
        Err(dberr) => match dberr {
            // NotFound error means authentication failed
            Error::NotFound => return Ok(None),
            // Any other error is unexpected
            _ => return Err(dberr),
        },
    };

    Ok(Some(user))
}
