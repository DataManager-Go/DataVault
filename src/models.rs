use super::schema::users;

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub disabled: bool,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a str,
}
