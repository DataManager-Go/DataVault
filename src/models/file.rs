use crate::{
    config::Config,
    handlers::requests::file::FileList,
    models::{self, namespace::Namespace, user::User},
    response_code::{diesel_option, Origin, RestError},
    schema::{self, files},
    utils::random_string,
    DbConnection,
};
use chrono::prelude::*;
use diesel::{
    dsl::count_star, pg::Pg, prelude::*, result::Error as DieselErr, PgTextExpressionMethods,
};
use models::attribute::{
    AttributeType::{Group, Tag},
    NewAttribute,
};
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
    pub fn create(self, db: &DbConnection) -> Result<File, diesel::result::Error> {
        use crate::schema::files::dsl::*;

        let (nid, nuploaded_at) = diesel::insert_into(files)
            .values(&self)
            .returning((id, uploaded_at))
            .get_result(db)?;

        Ok(File {
            id: nid,
            user_id: self.user_id,
            name: self.name,
            checksum: self.checksum,
            namespace_id: self.namespace_id,
            encryption: self.encryption,
            public_filename: self.public_filename,
            is_public: self.is_public,
            file_type: self.file_type,
            file_size: self.file_size,
            local_name: self.local_name,
            uploaded_at: nuploaded_at,
        })
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
            .map_err(|i| diesel_option(i, Origin::File))
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
    pub fn find_by_name(db: &DbConnection, f_name: &str, ns: i32) -> Result<File, DieselErr> {
        use crate::schema::files::dsl::*;
        files
            .filter(name.eq(f_name).and(namespace_id.eq(ns)))
            .first::<File>(db)
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
        Namespace::find_by_id(db, self.namespace_id)
    }

    /// Delete the file
    pub fn delete(&self, db: &DbConnection, config: &Config) -> Result<(), RestError> {
        // TODO shredder file

        // We need to delete associations first
        // otherwise db relations errors will occur
        attributes::delete_file_associations(db, self.id)?;

        // rm from DB
        diesel::delete(self).execute(db)?;

        // Delete local file. Ignore errors
        fs::remove_file(Path::new(&config.server.file_output_path).join(&self.local_name)).ok();

        Ok(())
    }

    /// Search for a file. Unset values are ignored
    pub fn find(&self, db: &DbConnection, ignore_ns: bool) -> Result<Vec<File>, RestError> {
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

    /// Search for a file
    pub fn search(
        db: &DbConnection,
        filter: &FileList,
        user: User,
    ) -> Result<Vec<(File, Namespace, Vec<Attribute>)>, RestError> {
        use crate::schema::files::dsl::*;

        let mut query = files
            // Join namespaces
            .left_join(schema::file_attributes::table)
            .left_join(
                schema::attributes::table
                    .on(schema::attributes::id.eq(schema::file_attributes::attribute_id)),
            )
            .inner_join(schema::namespaces::table)
            // Always filter by user_id
            .filter(user_id.eq(user.id))
            .into_boxed::<Pg>();

        let mut ns_id: Option<i32> = None;

        // Apply namespace filter
        if !filter.all_namespaces {
            let ns = Namespace::find_by_name(db, &filter.attributes.namespace, user.id)?
                .ok_or(RestError::NotFound)?;

            ns_id = Some(ns.id);

            query = query.filter(namespace_id.eq(ns.id));
        }

        // Apply name filter
        if !filter.name.is_empty() {
            query = query.filter(name.ilike(&filter.name));
        }

        use itertools::Itertools;

        let result = query.load::<(
            File,
            Option<models::file::attributes::FileAttribute>,
            Option<models::attribute::Attribute>,
            Namespace,
        )>(db)?;

        // Apply attribute filter
        let attrs_to_filter = if filter.attributes.groups.is_some()
            || filter.attributes.tags.is_some()
        {
            let tags = if let Some(ref tags) = filter.attributes.tags {
                NewAttribute::find_multi_by_name(&db, &tags, Tag, user.id, ns_id.unwrap_or(-1))?
            } else {
                vec![]
            };

            // Get and create groups
            let groups = if let Some(ref groups) = filter.attributes.groups {
                NewAttribute::find_multi_by_name(&db, &groups, Group, user.id, ns_id.unwrap_or(-1))?
            } else {
                vec![]
            };

            [tags, groups].concat().iter().map(|i| i.id).collect_vec()
        } else {
            vec![]
        };

        // Collect multiple files with same ID into one, with multiple all attributes
        let res: Vec<(File, Namespace, Vec<Attribute>)> = result
            .into_iter()
            .group_by(|i| i.0.id)
            .into_iter()
            .filter_map(|(_, mut file)| {
                let e = file.next().unwrap();
                let mut concatted_file: (File, Namespace, Vec<Attribute>) = (
                    e.0,
                    e.3,
                    // Create a new vector containing the attribute if exists
                    e.2.map(|i| vec![i]).unwrap_or_default(),
                );

                // Append all attributes from other results
                concatted_file.2.extend(file.filter_map(|i| i.2));

                // filter out attributes
                if !attrs_to_filter.is_empty() {
                    // Only return if concatted_file contains
                    // of the passed attributes
                    for file_attr in &concatted_file.2 {
                        if attrs_to_filter.contains(&file_attr.id) {
                            return Some(concatted_file);
                        }
                    }

                    None
                } else {
                    Some(concatted_file)
                }
            })
            .collect();

        Ok(res)
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
    use diesel::{dsl::exists, prelude::*, result::Error as DieselErr};

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

    /// Return true if an attribute has relations / is in use
    fn is_attribute_used(db: &DbConnection, aid: i32) -> Result<bool, RestError> {
        use crate::schema::file_attributes::dsl::*;
        Ok(diesel::select(exists(file_attributes.filter(attribute_id.eq(aid)))).get_result(db)?)
    }

    /// Delete a single association
    pub fn delete_association(db: &DbConnection, fid: i32, aid: i32) -> Result<(), RestError> {
        use crate::schema::file_attributes::dsl::*;

        diesel::delete(file_attributes)
            .filter(file_id.eq(fid).and(attribute_id.eq(aid)))
            .execute(db)?;

        // Delete unused attribute
        if !is_attribute_used(db, aid)? {
            super::super::attribute::delete(db, aid)?;
        }

        Ok(())
    }

    /// Delete all attribute associations for an attribute
    pub fn delete_attribute_associations(db: &DbConnection, aid: i32) -> Result<(), DieselErr> {
        use crate::schema::file_attributes::dsl::*;

        diesel::delete(file_attributes)
            .filter(attribute_id.eq(aid))
            .execute(db)?;

        Ok(())
    }

    /// Delete all attribute associations for a file
    pub fn delete_file_associations(db: &DbConnection, fid: i32) -> Result<(), RestError> {
        use crate::schema::file_attributes::dsl::*;

        // Get all attributes of the given file
        let attributes = get_file_attribute_ids(db, fid)?;

        if attributes.is_empty() {
            return Ok(());
        }

        diesel::delete(file_attributes)
            .filter(file_id.eq(fid))
            .execute(db)?;

        // Delete unused attributes
        for attribute in attributes {
            if !is_attribute_used(db, attribute)? {
                super::super::attribute::delete(db, attribute)?;
            }
        }

        Ok(())
    }
}
