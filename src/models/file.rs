use crate::response_code::{diesel_option, RestError};
use crate::schema::files;
use crate::{
    config::Config,
    models::{namespace::Namespace, user::User},
    DbConnection,
};
use async_std::{fs, path::Path};
use chrono::prelude::*;
use diesel::{dsl::count_star, prelude::*};

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
    pub async fn delete(&self, db: &DbConnection, config: &Config) -> Result<(), RestError> {
        // Delete local file
        // TODO shredder file
        fs::remove_file(Path::new(&config.server.file_output_path).join(&self.local_name)).await?;

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
        if let Ok(_) = files.filter(public_filename.eq(pub_name)).first::<File>(db) {
            return Err(RestError::AlreadyExists);
        }

        // Select proper public name
        if pub_name.is_empty() {
            self.public_filename = Some(crate::utils::random_string(25));
        } else {
            self.public_filename = Some(pub_name.to_string());
        }

        self.is_public = true;
        self.save(db)?;
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
