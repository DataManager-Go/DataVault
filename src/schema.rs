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
    namespaces (id) {
        id -> Int4,
        name -> Text,
        creator -> Int4,
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

joinable!(login_sessions -> users (user_id));
joinable!(namespaces -> users (creator));

allow_tables_to_appear_in_same_query!(
    login_sessions,
    namespaces,
    users,
);
