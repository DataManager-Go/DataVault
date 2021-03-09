table! {
    files (id) {
        id -> Int4,
        name -> Text,
        user_id -> Int4,
        local_name -> Text,
        uploaded_at -> Nullable<Timestamptz>,
        file_size -> Int8,
        file_type -> Text,
        is_public -> Nullable<Bool>,
        public_filename -> Nullable<Text>,
        namespace_id -> Int4,
        encryption -> Int4,
        checksum -> Text,
    }
}

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
        user_id -> Int4,
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

joinable!(files -> users (user_id));
joinable!(login_sessions -> users (user_id));
joinable!(namespaces -> users (user_id));

allow_tables_to_appear_in_same_query!(
    files,
    login_sessions,
    namespaces,
    users,
);
