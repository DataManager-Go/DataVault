table! {
    login_sessions (id) {
        id -> Int4,
        user_id -> Int4,
        token -> Text,
        requests -> Int8,
        machine_id -> Nullable<Text>,
    }
}

table! {
    users (id) {
        id -> Int4,
        username -> Text,
        password -> Text,
        disabled -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    login_sessions,
    users,
);
