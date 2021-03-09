use crate::schema::*;
use crate::{
    models::{namespace::Namespace, user::User},
    DbConnection,
};
use chrono::prelude::*;
use diesel::prelude::*;

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(User)]
#[belongs_to(Namespace)]
pub struct File {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub local_name: String,
    pub uploaded_at: DateTime<Utc>,
    pub file_size: u64,
    pub file_type: String,
    pub is_public: bool,
    pub public_filename: Option<String>,
    pub namespace_id: i32,
    pub encryption: String,
    pub checksum: String,
}

#[derive(Insertable)]
#[table_name = "files"]
pub struct NewFile {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub local_name: String,
    pub uploaded_at: NaiveDateTime,
    pub file_size: i64,
    pub file_type: String,
    pub is_public: bool,
    pub public_filename: String,
    pub namespace_id: i32,
    pub encryption: i32,
    pub checksum: String,
}

impl NewFile {
    pub fn create(&self, db: &DbConnection) -> Result<(), diesel::result::Error> {
        use crate::schema::files::dsl::*;
        diesel::insert_into(files).values(self).execute(db)?;
        Ok(())
    }
}
