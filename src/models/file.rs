use crate::response_code::{diesel_option, RestError};
use crate::schema::files;
use crate::{
    models::{namespace::Namespace, user::User},
    DbConnection,
};
use chrono::prelude::*;
use diesel::{dsl::count_star, prelude::*};

#[derive(Identifiable, Queryable, Associations, Debug, AsChangeset)]
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
    pub fn find_by_id(idd: i32, db: &DbConnection) -> Result<File, RestError> {
        use crate::schema::files::dsl::*;
        files.find(idd).first::<File>(db).map_err(diesel_option)
    }

    pub fn find_by_name_count(
        db: &DbConnection,
        f_name: String,
        ns: i32,
    ) -> Result<i64, RestError> {
        use crate::schema::files::dsl::*;

        files
            .filter(name.eq(f_name).and(namespace_id.eq(ns)))
            .select(count_star())
            .first(db)
            .map_err(|i| i.into())
    }

    pub fn find_by_name(db: &DbConnection, f_name: String, ns: i32) -> Result<File, RestError> {
        use crate::schema::files::dsl::*;
        files
            .filter(name.eq(f_name).and(namespace_id.eq(ns)))
            .first::<File>(db)
            .map_err(diesel_option)
    }

    /// Saves an existing file
    pub fn save(&self, db: &DbConnection) -> Result<(), RestError> {
        use crate::schema::files::dsl::*;
        diesel::update(files)
            .set(self)
            .filter(id.eq(self.id))
            .execute(db)?;
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
