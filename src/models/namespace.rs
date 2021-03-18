use crate::{
    config::Config,
    models::user::User,
    response_code::{self, RestError},
};
use crate::{schema::*, DbConnection};
use diesel::prelude::*;
use diesel::result::{Error as DieselErr, Error::NotFound};
use serde::Serialize;

use super::file::File;

/// A namespace represents a abstraction between multiple files.
/// Each namespace, identified by its per user unique name can
/// only exists once.
#[derive(Identifiable, Queryable, Clone, Debug, Default, Serialize, PartialEq, Associations)]
#[belongs_to(User)]
pub struct Namespace {
    pub id: i32,
    pub name: String,
    pub user_id: i32,
}

#[derive(Insertable)]
#[table_name = "namespaces"]
pub struct CreateNamespace<'a> {
    pub name: &'a str,
    pub user_id: i32,
}

impl<'a> CreateNamespace<'a> {
    /// Creates a new CreateNamespace object. The name
    /// must not be prepended with the users prefix.
    pub fn new(name: &'a str, user_id: i32) -> CreateNamespace<'a> {
        CreateNamespace { name, user_id }
    }

    /// Creates a new namespace owned by the user whose ID was passed
    pub fn create(&self, db: &DbConnection) -> Result<(), RestError> {
        // Check whether namespace exists or not
        if Namespace::find_by_name(db, self.name, self.user_id)?.is_some() {
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
    /// Returns true if the name matches with
    // the name of a default namespace
    pub fn is_default_name(ns_name: &str) -> bool {
        ns_name.to_lowercase() == "default"
    }

    /// Returns true if the namespace is a default namespace
    pub fn is_default(&self) -> bool {
        Namespace::is_default_name(&self.name)
    }

    /// Find a namespace by its id
    pub fn find_by_id(db: &DbConnection, idd: i32) -> Result<Namespace, RestError> {
        use crate::schema::namespaces::dsl::*;

        namespaces
            .find(idd)
            .first(db)
            .map_err(response_code::diesel_option)
    }

    /// Find a namespace by its name
    /// The creator has to be passed in order
    /// to prevent unauthorized access
    pub fn find_by_name(
        db: &DbConnection,
        ns_name: &str,
        creator: i32,
    ) -> Result<Option<Namespace>, DieselErr> {
        use crate::schema::namespaces::dsl::*;

        let res = namespaces
            .filter(user_id.eq(creator).and(name.eq(ns_name)))
            .first(db);

        if let Err(NotFound) = res {
            return Ok(None);
        }

        res.map(Some)
    }

    /// List all namespaces of a user
    pub fn list(db: &DbConnection, user: &User) -> Result<Vec<Namespace>, RestError> {
        Namespace::belonging_to(user)
            .load::<Namespace>(db)
            .map_err(|i| i.into())
    }

    /// Delete a namespace
    pub fn delete(&self, db: &DbConnection, config: &Config) -> Result<(), RestError> {
        use crate::schema::namespaces::dsl::*;

        // Don't allow deleting 'default' namespace
        if self.name == "default" {
            return Err(RestError::IllegalOperation);
        }

        // Delete all namespace assigned files
        for file in self.files(db)? {
            file.delete(db, config)?;
        }

        // Delet namespace from database
        diesel::delete(namespaces)
            .filter(id.eq(self.id))
            .execute(db)?;

        Ok(())
    }

    /// Get a list of all files in the current namespace
    pub fn files(&self, db: &DbConnection) -> Result<Vec<File>, diesel::result::Error> {
        use crate::schema::files::dsl::*;
        files.filter(namespace_id.eq(self.id)).load::<File>(db)
    }

    /// Rename a namespace
    pub fn rename(&self, db: &DbConnection, new_name: &str) -> Result<(), RestError> {
        use crate::schema::namespaces::dsl::*;

        diesel::update(namespaces)
            .set(name.eq(new_name))
            .filter(id.eq(self.id))
            .execute(db)?;

        Ok(())
    }
}
