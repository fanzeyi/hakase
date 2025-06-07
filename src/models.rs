use super::schema::url;
use chrono::NaiveDateTime;
use diesel::Insertable;
use diesel::Queryable;

#[derive(Queryable, Debug)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Url {
    pub id: i32,
    pub code: String,
    pub myurl: String,
    pub create_time: NaiveDateTime,
    pub count: i32,
}

#[derive(Insertable)]
#[diesel(table_name = url)]
pub struct NewUrl<'a> {
    pub myurl: &'a str,
    pub code: &'a str,
}
