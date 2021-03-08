use crate::{
    config::Config,
    models::{NewUser, User},
    response_code::{self, RestError, Success, SUCCESS},
    utils, DbConnection, DbPool,
};

use actix_web::web::{self, Json};
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind, Error::DatabaseError},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CredentialsRequest {
    mid: Option<String>,
    username: String,
    pass: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    token: String,
    ns: String,
}

impl CredentialsRequest {
    // Returns true if one value is empty
    pub fn has_empty(&self) -> bool {
        self.username.is_empty() || self.pass.is_empty()
    }
}

/// Endpoint for registering new users
pub async fn ep_register(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req: web::Json<CredentialsRequest>,
) -> Result<Json<Success>, RestError> {
    // Don't allow the registration ep if disabled in config
    config
        .server
        .allow_registration
        .then(|| false)
        .ok_or(RestError::Forbidden)?;

    if req.has_empty() {
        return Err(RestError::BadRequest);
    }

    let db = pool.get()?;

    web::block(move || register(&db, &req.username, &req.pass)).await?;

    Ok(SUCCESS)
}

/// Endpoint for loggin in users
pub async fn ep_login(
    pool: web::Data<DbPool>,
    req: web::Json<CredentialsRequest>,
) -> Result<Json<LoginResponse>, RestError> {
    if req.has_empty() {
        return Err(RestError::BadRequest);
    }

    let db = pool.get()?;

    let user = find_user_by_name(&db, &req.username)?;
    let cloned_user = user.clone();

    let token = web::block(move || login(&db, &req.pass, &req.mid, &cloned_user)).await?;

    Ok(Json(LoginResponse {
        token,
        ns: user.get_default_ns(),
    }))
}

/// Register a new user
pub fn register(db: &DbConnection, username: &str, password: &str) -> Result<(), RestError> {
    use crate::schema::users;

    let password = &utils::sha512(&[&username, &password]);

    if let Err(err) = diesel::insert_into(users::table)
        .values(&NewUser { username, password })
        .execute(db)
    {
        return Err(match err {
            DatabaseError(DatabaseErrorKind::UniqueViolation, _) => RestError::UserExists,
            _ => RestError::Unknown,
        });
    }

    Ok(())
}

/// Create a user session
pub fn login(
    db: &DbConnection,
    password: &String,
    mid: &Option<String>,
    user: &User,
) -> Result<String, RestError> {
    use crate::{
        models::NewLoginSession,
        schema::login_sessions::{self, dsl::*},
    };

    // Validate password
    if user.password != *password {
        return Err(RestError::Unauthorized);
    }

    // Clear old session(s)
    if let Some(ref mid) = mid {
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
    diesel::insert_into(login_sessions::table)
        .values(&new_token)
        .execute(db)?;

    return Ok(new_token.token);
}

// Find a user by its ID
pub fn find_user_by_name(db: &DbConnection, name: &str) -> Result<User, RestError> {
    use crate::schema::users::dsl::*;

    Ok(users
        .filter(username.eq(name))
        .first::<User>(db)
        .map_err(response_code::login_error)?)
}
