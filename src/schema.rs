use diesel::table;

table! {
    url (id) {
        id -> Integer,
        code -> Text,
        #[sql_name="url"]
        myurl -> Text,
        create_time -> Timestamp,
        count -> Integer,
    }
}
