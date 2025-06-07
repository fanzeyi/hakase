use chrono::NaiveDateTime;
use rusqlite::{Row, Result};

#[derive(Debug)]
pub struct Url {
    pub id: i32,
    pub code: String,
    pub myurl: String,
    pub create_time: NaiveDateTime,
    pub count: i32,
}

impl Url {
    pub fn from_row(row: &Row) -> Result<Url> {
        Ok(Url {
            id: row.get(0)?,
            code: row.get(1)?,
            myurl: row.get(2)?,
            create_time: row.get(3)?,
            count: row.get(4)?,
        })
    }
}

pub struct NewUrl<'a> {
    pub myurl: &'a str,
    pub code: &'a str,
}

impl<'a> NewUrl<'a> {
    pub fn new(myurl: &'a str, code: &'a str) -> Self {
        NewUrl { myurl, code }
    }
}