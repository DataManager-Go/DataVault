use super::{namespace::Namespace, user::User};

use crate::{schema::attributes, DbConnection};

use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    prelude::*,
    result::Error as DieselErr,
    serialize::{self, Output, ToSql},
    sql_types::*,
};
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Identifiable, Queryable, Associations, Debug, AsChangeset, Clone)]
#[belongs_to(User)]
#[changeset_options(treat_none_as_null = "true")]
#[belongs_to(Namespace)]
pub struct Attribute {
    pub id: i32,
    pub type_: AttributeType,
    pub name: String,
    pub user_id: i32,
    pub namespace_id: i32,
}

#[derive(Insertable, Debug)]
#[table_name = "attributes"]
pub struct NewAttribute {
    pub type_: AttributeType,
    pub name: String,
    pub user_id: i32,
    pub namespace_id: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, AsExpression, FromSqlRow, PartialEq)]
#[sql_type = "SmallInt"]
pub enum AttributeType {
    Group,
    Tag,
}

impl NewAttribute {
    /// Create a new NewAttribute object instance
    pub fn new(name: &str, type_: AttributeType, user_id: i32, namespace_id: i32) -> Self {
        Self {
            type_,
            name: name.to_owned(),
            user_id,
            namespace_id,
        }
    }

    /// Create a new attribute and return it
    pub fn create(&self, db: &DbConnection) -> Result<Attribute, DieselErr> {
        use crate::schema::attributes::dsl::*;

        let attr_id = diesel::insert_into(attributes)
            .values(self)
            .returning(id)
            .get_result(db)?;

        Ok(Attribute {
            id: attr_id,
            user_id: self.user_id,
            name: self.name.clone(),
            namespace_id: self.namespace_id,
            type_: self.type_,
        })
    }

    /// Find the passed NewAttribute in DB and returns Some(Attribute) if found
    pub fn find(&self, db: &DbConnection) -> Result<Option<Attribute>, DieselErr> {
        use crate::schema::attributes::dsl::*;
        let iid: Option<Attribute> = match attributes
            .filter(
                name.eq(&self.name)
                    .and(user_id.eq(self.user_id))
                    .and(namespace_id.eq(self.namespace_id))
                    .and(type_.eq(self.type_)),
            )
            .limit(1)
            .get_result::<Attribute>(db)
        {
            Ok(idd) => Some(idd),
            Err(err) => match err {
                DieselErr::NotFound => None,
                _ => return Err(err),
            },
        };

        Ok(iid)
    }

    /// Returns Ok(true) if the attribute exists in DB
    pub fn exists(&self, db: &DbConnection) -> Result<bool, DieselErr> {
        Ok(self.find(db)?.is_some())
    }

    /// Finds all matching Attributes
    pub fn find_by_name(
        db: &DbConnection,
        items: &[String],
        typ: AttributeType,
        uid: i32,
        ns_id: i32,
    ) -> Result<Vec<Attribute>, DieselErr> {
        use crate::schema::attributes::dsl::*;

        let res = items
            .iter()
            .map(|item| {
                attributes
                    .filter(
                        name.eq(item)
                            .and(namespace_id.eq(ns_id))
                            .and(user_id.eq(uid))
                            .and(type_.eq(typ)),
                    )
                    .limit(1)
                    .get_result(db)
            })
            .collect::<Result<Vec<Attribute>, DieselErr>>()?;

        Ok(res)
    }

    /// Create all missing attributes of type type_
    pub fn find_and_create(
        db: &DbConnection,
        items: &[String],
        type_: AttributeType,
        user_id: i32,
        namespace_id: i32,
    ) -> Result<Vec<Attribute>, DieselErr> {
        Ok(items
            .iter()
            .map(|item| -> Result<Attribute, DieselErr> {
                let attr = NewAttribute {
                    user_id,
                    namespace_id,
                    name: item.clone(),
                    type_,
                };

                let found = attr.find(db)?;

                match found {
                    Some(attr) => Ok(attr),
                    None => Ok(attr.create(db)?),
                }
            })
            .collect::<Result<Vec<Attribute>, DieselErr>>()?)
    }
}

/// Delete an attribute by its ID
pub fn delete(db: &DbConnection, attr_id: i32) -> Result<(), DieselErr> {
    use crate::schema::attributes::dsl::*;
    diesel::delete(attributes)
        .filter(id.eq(attr_id))
        .execute(db)?;
    Ok(())
}

impl<DB: Backend> ToSql<SmallInt, DB> for AttributeType
where
    i16: ToSql<SmallInt, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
    where
        W: io::Write,
    {
        match *self {
            AttributeType::Tag => 1,
            AttributeType::Group => 2,
        }
        .to_sql(out)
    }
}

impl<DB: Backend> FromSql<SmallInt, DB> for AttributeType
where
    i16: FromSql<SmallInt, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let v = i16::from_sql(bytes)?;
        Ok(match v {
            1 => AttributeType::Tag,
            2 => AttributeType::Group,
            _ => return Err("Invalid AttributeType".into()),
        })
    }
}
