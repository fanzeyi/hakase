use diesel::table;

table! {
    url (id) {
        id -> Integer,
        code -> Varchar,
        #[sql_name="url"]
        myurl -> Varchar,
        create_time -> Datetime,
        count -> Integer,
    }
}
