table! {
    attributes (id) {
        id -> Int4,
        #[sql_name = "type"]
        type_ -> Int2,
        name -> Text,
        namespace_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    file_attributes (id) {
        id -> Int4,
        file_id -> Int4,
        attribute_id -> Int4,
    }
}

table! {
    files (id) {
        id -> Int4,
        name -> Text,
        user_id -> Int4,
        local_name -> Text,
        uploaded_at -> Timestamptz,
        file_size -> Int8,
        file_type -> Text,
        is_public -> Bool,
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

joinable!(attributes -> namespaces (namespace_id));
joinable!(attributes -> users (user_id));
joinable!(file_attributes -> attributes (attribute_id));
joinable!(file_attributes -> files (file_id));
joinable!(files -> namespaces (namespace_id));
joinable!(files -> users (user_id));
joinable!(login_sessions -> users (user_id));
joinable!(namespaces -> users (user_id));

allow_tables_to_appear_in_same_query!(
    attributes,
    file_attributes,
    files,
    login_sessions,
    namespaces,
    users,
);
