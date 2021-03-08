use crate::response_code::RestError;
use crate::{schema::*, DbConnection};
use diesel::prelude::*;
use diesel::result::Error::NotFound;
use serde::Serialize;

/// A namespace represents a abstraction between multiple files.
/// Each namespace, identified by its per user unique name can
/// only exists once.
#[derive(Queryable, Clone, Debug, Default, Serialize)]
pub struct Namespace {
    pub id: i32,
    pub name: String,
    pub creator: i32,
}

#[derive(Insertable)]
#[table_name = "namespaces"]
pub struct CreateNamespace<'a> {
    pub name: &'a str,
    pub creator: i32,
}

impl<'a> CreateNamespace<'a> {
    /// Creates a new CreateNamespace object. The name
    /// must not be prepended with the users prefix.
    pub fn new(name: &'a str, creator: i32) -> CreateNamespace<'a> {
        CreateNamespace { name, creator }
    }

    /// Creates a new namespace owned by the user whose ID was passed
    pub fn create(&self, db: &DbConnection) -> Result<(), RestError> {
        // Check whether namespace exists or not
        if Namespace::find_by_name(db, self.name, self.creator)?.is_some() {
            return Err(RestError::AlreadyExists);
        }

        // Insert new namespace
        diesel::insert_into(namespaces::table)
            .values(self)
            .execute(db)?;

        Ok(())
    }
}

impl Namespace {
    /// Find a namespace by its name
    pub fn find_by_name(
        db: &DbConnection,
        ns_name: &str,
        user_id: i32,
    ) -> Result<Option<Namespace>, RestError> {
        use crate::schema::namespaces::dsl::*;

        let res = namespaces
            .filter(creator.eq(user_id).and(name.eq(ns_name)))
            .first(db);

        if let Err(NotFound) = res {
            return Ok(None);
        }

        res.map(Some).map_err(|i| i.into())
    }

    /// List all namespaces of a user
    pub fn list(db: &DbConnection, user_id: i32) -> Result<Vec<Namespace>, RestError> {
        use crate::schema::namespaces::dsl::*;

        namespaces
            .filter(creator.eq(user_id))
            .load::<Namespace>(db)
            .map_err(|i| i.into())
    }
}
