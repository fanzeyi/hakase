
use std::time::SystemTime;
use super::schema::url;

#[derive(Queryable)]
pub struct Url {
    pub id: i32,
    pub url: String,
    pub code: String,
    pub count: i32,
    pub create_time: SystemTime,
}

#[derive(Insertable)]
#[table_name="url"]
pub struct NewUrl<'a> {
    pub myurl: &'a str,
    pub code: &'a str,
}
