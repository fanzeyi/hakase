
use chrono::NaiveDateTime;
use super::schema::url;

#[derive(Queryable, Debug)]
pub struct Url {
    pub id: i32,
    pub code: String,
    pub url: String,
    pub create_time: NaiveDateTime,
    pub count: i32,
}

#[derive(Insertable)]
#[table_name="url"]
pub struct NewUrl<'a> {
    pub myurl: &'a str,
    pub code: &'a str,
}
