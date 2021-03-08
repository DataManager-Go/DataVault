use crate::schema::*;

#[derive(Queryable, Clone, Debug, Default)]
pub struct Namespace {
    pub id: i32,
    pub name: String,
    pub creator: i32,
}

#[derive(Insertable)]
#[table_name = "namespaces"]
pub struct NewNamespace<'a> {
    pub name: &'a str,
    pub creator: i32,
}
