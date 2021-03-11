use crate::{models::user::User, response_code::{self,RestError}};
use crate::{schema::*, DbConnection};
use diesel::prelude::*;
use diesel::result::Error::NotFound;
use serde::Serialize;

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
    /// Find a namespace by its id
    pub fn find_by_id(
        db: &DbConnection,
        idd: i32,
    ) -> Result<Namespace, RestError> {
        use crate::schema::namespaces::dsl::*;

        namespaces
            .find(idd)
            .first(db).map_err(response_code::diesel_option)
    }

    /// Find a namespace by its name
    pub fn find_by_name(
        db: &DbConnection,
        ns_name: &str,
        creator: i32,
    ) -> Result<Option<Namespace>, RestError> {
        use crate::schema::namespaces::dsl::*;

        let res = namespaces
            .filter(user_id.eq(creator).and(name.eq(ns_name)))
            .first(db);

        if let Err(NotFound) = res {
            return Ok(None);
        }

        res.map(Some).map_err(|i| i.into())
    }

    /// List all namespaces of a user
    pub fn list(db: &DbConnection, user: &User) -> Result<Vec<Namespace>, RestError> {
        Namespace::belonging_to(user)
            .load::<Namespace>(db)
            .map_err(|i| i.into())
    }

    /// Delete a namespace
    pub fn delete(&self, db: &DbConnection) -> Result<(), RestError> {
        use crate::schema::namespaces::dsl::*;

        // Don't allow deleting 'default' namespace
        if self.name == "default" {
            return Err(RestError::IllegalOperation);
        }

        // TODO delete namespaces files, tags and groups here as well

        diesel::delete(namespaces)
            .filter(id.eq(self.id))
            .execute(db)?;

        Ok(())
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
