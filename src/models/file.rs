use crate::schema::files;
use crate::{
    config::Config,
    models::{namespace::Namespace, user::User},
    DbConnection,
};
use crate::{
    response_code::{diesel_option, RestError},
    utils::random_string,
};
use chrono::prelude::*;
use diesel::{dsl::count_star, prelude::*};
use std::{fs, path::Path};

use super::attribute::Attribute;

#[derive(Identifiable, Queryable, Associations, Debug, AsChangeset, Clone)]
#[belongs_to(User)]
#[changeset_options(treat_none_as_null = "true")]
#[belongs_to(Namespace)]
pub struct File {
    pub id: i32,
    pub name: String,
    pub user_id: i32,
    pub local_name: String,
    pub uploaded_at: DateTime<Utc>,
    pub file_size: i64,
    pub file_type: String,
    pub is_public: bool,
    pub public_filename: Option<String>,
    pub namespace_id: i32,
    pub encryption: i32,
    pub checksum: String,
}

impl Default for File {
    fn default() -> Self {
        File {
            id: i32::default(),
            user_id: i32::default(),
            local_name: String::default(),
            name: String::default(),
            uploaded_at: Utc::now(),
            file_size: i64::default(),
            file_type: String::default(),
            is_public: bool::default(),
            public_filename: None,
            namespace_id: i32::default(),
            encryption: i32::default(),
            checksum: String::default(),
        }
    }
}

#[derive(Insertable, Default, Debug)]
#[table_name = "files"]
pub struct NewFile {
    pub name: String,
    pub user_id: i32,
    pub local_name: String,
    pub file_size: i64,
    pub file_type: String,
    pub is_public: bool,
    pub public_filename: Option<String>,
    pub namespace_id: i32,
    pub encryption: i32,
    pub checksum: String,
}

impl NewFile {
    /// Create a new file
    pub fn create(&self, db: &DbConnection) -> Result<i32, diesel::result::Error> {
        use crate::schema::files::dsl::*;
        diesel::insert_into(files)
            .values(self)
            .returning(id)
            .get_result(db)
    }
}

impl File {
    /// Find a file by its id.
    /// The user owner has to be passed as well, in order
    /// To prevent unauthorized access to files
    pub fn find_by_id(db: &DbConnection, idd: i32, uid: i32) -> Result<File, RestError> {
        use crate::schema::files::dsl::*;
        files
            .find(idd)
            .filter(user_id.eq(uid))
            .first::<File>(db)
            .map_err(diesel_option)
    }

    /// Get the count of files which can
    // be found by the passed name and ns
    pub fn find_by_name_count(db: &DbConnection, f_name: &str, ns: i32) -> Result<i64, RestError> {
        use crate::schema::files::dsl::*;

        files
            .filter(name.eq(f_name).and(namespace_id.eq(ns)))
            .select(count_star())
            .first(db)
            .map_err(|i| i.into())
    }

    /// Find a file by its name and namespace
    pub fn find_by_name(db: &DbConnection, f_name: &str, ns: i32) -> Result<File, RestError> {
        use crate::schema::files::dsl::*;
        files
            .filter(name.eq(f_name).and(namespace_id.eq(ns)))
            .first::<File>(db)
            .map_err(diesel_option)
    }

    /// Saves an existing file
    pub fn save(&self, db: &DbConnection) -> Result<(), diesel::result::Error> {
        use crate::schema::files::dsl::*;
        diesel::update(files)
            .set(self)
            .filter(id.eq(self.id))
            .execute(db)?;
        Ok(())
    }

    /// Get the namespace of the file
    pub fn namespace(&self, db: &DbConnection) -> Result<Namespace, RestError> {
        Ok(Namespace::find_by_id(db, self.namespace_id)?)
    }

    /// Delete the file
    pub fn delete(&self, db: &DbConnection, config: &Config) -> Result<(), RestError> {
        // TODO shredder file

        // We need to delete associations first
        // otherwise db relations errors will occur
        attributes::delete_associations(db, self.id)?;

        // Delete local file
        fs::remove_file(Path::new(&config.server.file_output_path).join(&self.local_name))?;

        // rm from DB
        diesel::delete(self).execute(db)?;
        Ok(())
    }

    /// Search for a file. Unset values are ignored
    pub fn search(&self, db: &DbConnection, ignore_ns: bool) -> Result<Vec<File>, RestError> {
        use crate::schema::files::dsl::*;
        let mut query = {
            if self.id > 0 {
                files.filter(id.eq(self.id)).into_boxed()
            } else {
                files.filter(name.ilike(&self.name)).into_boxed()
            }
        };

        // Ensure only to select files which
        // are in some way associated with a user
        if self.namespace_id > 0 && !ignore_ns {
            query = query.filter(namespace_id.eq(self.namespace_id));
        } else {
            query = query.filter(user_id.eq(self.user_id));
        }

        query.load::<File>(db).map_err(|i| i.into())
    }

    /// Make a file public. If an empty name is provided, a random
    /// one will be generated. The file must be mutable in order
    /// to set the public_filename field to the new public name
    pub fn publish(&mut self, db: &DbConnection, pub_name: &str) -> Result<(), RestError> {
        // check whether the public name already exists
        use crate::schema::files::dsl::*;
        if files
            .filter(public_filename.eq(pub_name))
            .first::<File>(db)
            .is_ok()
        {
            return Err(RestError::AlreadyExists);
        }

        // Select proper public name
        self.public_filename = if pub_name.is_empty() {
            Some(random_string(25))
        } else {
            Some(pub_name.to_string())
        };

        self.is_public = true;
        self.save(db)?;
        Ok(())
    }

    /// Add a set of attributes to the file.
    /// Skips already added attributes
    pub fn add_attributes(
        &self,
        db: &DbConnection,
        attributes: Vec<Attribute>,
    ) -> Result<(), RestError> {
        // Get list of already existing attributes
        let existing_ids = attributes::get_file_attribute_ids(db, self.id)?;

        // Create vec with not yet added attributes
        let addition_needed: Vec<Attribute> = attributes
            .into_iter()
            .filter(|i| !existing_ids.contains(&i.id))
            .collect();

        if addition_needed.is_empty() {
            return Ok(());
        }

        attributes::add_atttributes(db, self.id, &addition_needed)?;

        Ok(())
    }

    /// Add a set of attributes to the file.
    /// Skips already added attributes
    pub fn remove_attributes(
        &self,
        db: &DbConnection,
        attributes: Vec<Attribute>,
    ) -> Result<(), RestError> {
        for attribute in attributes {
            attributes::delete_association(db, self.id, attribute.id)?;
        }

        Ok(())
    }
}

impl Into<NewFile> for File {
    fn into(self) -> NewFile {
        NewFile {
            name: self.name,
            user_id: self.user_id,
            local_name: self.local_name,
            file_size: self.file_size,
            file_type: self.file_type,
            is_public: self.is_public,
            public_filename: self.public_filename,
            namespace_id: self.namespace_id,
            encryption: self.encryption,
            checksum: self.checksum,
        }
    }
}

/// Attribute cross join table logic m:m
pub mod attributes {
    use crate::models::file::File;
    use crate::schema::*;
    use crate::{models::attribute::Attribute, response_code::RestError, DbConnection};
    use diesel::{dsl::exists, prelude::*};

    #[derive(Identifiable, Queryable, Associations, Debug, AsChangeset, Clone)]
    #[belongs_to(Attribute)]
    #[belongs_to(File)]
    #[changeset_options(treat_none_as_null = "true")]
    pub struct FileAttribute {
        pub id: i32,
        pub file_id: i32,
        pub attribute_id: i32,
    }

    #[derive(Insertable, Default, Debug, Copy, Clone)]
    #[table_name = "file_attributes"]
    pub struct NewFileAttribute {
        pub file_id: i32,
        pub attribute_id: i32,
    }

    /// Get the attribute_ids associated to a file id
    pub fn get_file_attribute_ids(db: &DbConnection, fid: i32) -> Result<Vec<i32>, RestError> {
        use crate::schema::file_attributes::dsl::*;

        Ok(file_attributes
            .filter(file_id.eq(fid))
            .select(attribute_id)
            .load::<i32>(db)?)
    }

    /// Add attributes (references) for a file
    pub fn add_atttributes(
        db: &DbConnection,
        fid: i32,
        attributes: &[Attribute],
    ) -> Result<(), RestError> {
        use crate::schema::file_attributes::dsl::*;

        // Create vec of NewFileAttribute which allow a bulk insert
        let to_insert: Vec<NewFileAttribute> = attributes
            .iter()
            .map(|i| NewFileAttribute {
                file_id: fid,
                attribute_id: i.id,
            })
            .collect();

        diesel::insert_into(file_attributes)
            .values(to_insert)
            .execute(db)?;

        Ok(())
    }

    /// Delete a single association between a file and a tag
    pub fn delete_association(db: &DbConnection, fid: i32, aid: i32) -> Result<(), RestError> {
        use crate::schema::file_attributes::dsl::*;

        diesel::delete(file_attributes)
            .filter(file_id.eq(fid).and(attribute_id.eq(aid)))
            .execute(db)?;

        if !diesel::select(exists(file_attributes.filter(attribute_id.eq(aid)))).get_result(db)? {
            // Delete unused attribute
            super::super::attribute::delete(db, aid)?;
        }

        Ok(())
    }

    /// Delete all attribute associations for a file
    pub fn delete_associations(db: &DbConnection, fid: i32) -> Result<(), RestError> {
        use crate::schema::file_attributes::dsl::*;

        diesel::delete(file_attributes)
            .filter(file_id.eq(fid))
            .execute(db)?;

        Ok(())
    }
}
