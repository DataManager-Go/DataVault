use crate::{
    response_code::{self, RestError},
    utils, DbConnection,
};

use crate::schema::*;
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error::DatabaseError},
};

#[derive(Queryable, Clone, Debug, Default)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub disabled: bool,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub password: String,
}

impl User {
    // Create a NewUser object
    pub fn new(username: String, password: String) -> NewUser {
        NewUser { username, password }
    }

    // Find a user by its Name
    pub fn find_by_name(db: &DbConnection, name: &str) -> Result<User, RestError> {
        use crate::schema::users::dsl::*;

        Ok(users
            .filter(username.eq(name))
            .first::<User>(db)
            .map_err(response_code::login_error)?)
    }

    // Find a user by its ID
    pub fn find_by_id(db: &DbConnection, user_id: i32) -> Result<User, RestError> {
        use crate::schema::users::dsl::*;

        Ok(users
            .filter(id.eq(user_id))
            .first::<User>(db)
            .map_err(response_code::login_error)?)
    }

    /// Create a new user session
    pub fn login(
        db: &DbConnection,
        username: &str,
        password: &str,
        mid: &Option<String>,
    ) -> Result<String, RestError> {
        use crate::{models::login_session::NewLoginSession, schema::login_sessions::dsl::*};

        let user = Self::find_by_name(&db, username)?;

        if user.disabled {
            return Err(RestError::UserDisabled);
        }

        // Salt & validate password
        if user.password != utils::hash_pw(username, password) {
            return Err(RestError::Unauthorized);
        }

        // Clear old session(s)
        if let Some(mid) = mid {
            diesel::delete(
                login_sessions.filter(
                    user_id
                        .eq(user.id)
                        .and(machine_id.nullable().is_not_null())
                        .and(machine_id.eq(mid)),
                ),
            )
            .execute(db)?;
        }

        // Generate new token
        let new_token = NewLoginSession {
            token: utils::random_string(60),
            machine_id: mid.clone(),
            user_id: user.id,
        };

        // Insert new token
        diesel::insert_into(login_sessions)
            .values(&new_token)
            .execute(db)?;

        Ok(new_token.token)
    }
}

impl NewUser {
    pub fn get_password_hashed(&self) -> String {
        crate::utils::hash_pw(&self.username, &self.password)
    }

    /// Register a new user
    pub fn create(self, db: &DbConnection) -> Result<Self, RestError> {
        let user = NewUser {
            password: self.get_password_hashed(),
            username: self.username,
        };

        if let Err(err) = diesel::insert_into(users::table).values(&user).execute(db) {
            return Err(match err {
                DatabaseError(DatabaseErrorKind::UniqueViolation, _) => RestError::AlreadyExists,
                _ => RestError::Unknown,
            });
        }

        Ok(user)
    }
}
