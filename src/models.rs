use chrono::NaiveDateTime;
use rusqlite::{Result, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Url {
    pub id: i32,
    pub code: String,
    pub url: String,
    pub create_time: NaiveDateTime,
    pub count: i32,
}

impl Url {
    pub fn from_row(row: &Row) -> Result<Url> {
        Ok(Url {
            id: row.get(0)?,
            code: row.get(1)?,
            url: row.get(2)?,
            create_time: row.get(3)?,
            count: row.get(4)?,
        })
    }
}

pub struct NewUrl<'a> {
    pub url: &'a str,
    pub code: &'a str,
}

impl<'a> NewUrl<'a> {
    pub fn new(url: &'a str, code: &'a str) -> Self {
        NewUrl { url, code }
    }
}
