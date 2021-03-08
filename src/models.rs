use super::schema::*;

#[derive(Queryable, Clone, Debug)]
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

#[derive(Queryable)]
pub struct LoginSession {
    pub id: i32,
    pub user_id: i32,
    pub token: i32,
    pub requests: i64,
    pub machine_id: Option<String>,
}

#[derive(Insertable)]
#[table_name = "login_sessions"]
pub struct NewLoginSession {
    pub user_id: i32,
    pub token: String,
    pub machine_id: Option<String>,
}

impl User {
    pub fn get_default_ns(&self) -> String {
        format!("{}_default", self.username)
    }
}
