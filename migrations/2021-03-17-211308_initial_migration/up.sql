CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username text NOT NULL,
    password text NOT NULL,
    disabled boolean NOT NULL DEFAULT False,
    UNIQUE(username)
);
CREATE TABLE login_sessions (
    id SERIAL PRIMARY KEY,
    user_id integer NOT NULL,
    token text NOT NULL,
    requests bigint NOT NULL DEFAULT 0,
    machine_id text,
    UNIQUE(token),
    foreign key (user_id) references users(id)
);
CREATE TABLE namespaces (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    user_id integer NOT NULL,
    foreign key (user_id) references users(id)
);
CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    user_id integer NOT NULL,
    local_name text NOT NULL,
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    file_size bigint NOT NULL,
    file_type text NOT NULL,
    is_public boolean DEFAULT false NOT NULL,
    public_filename text,
    namespace_id integer NOT NULL,
    encryption integer NOT NULL DEFAULT 0,
    checksum text NOT NULL,
    unique(local_name),
    foreign key (user_id) references users(id),
    foreign key (namespace_id) references namespaces(id)
);
ALTER SEQUENCE files_id_seq RESTART WITH 1;
CREATE TABLE attributes (
    id SERIAL PRIMARY KEY,
    type int2 NOT NULL,
    name text NOT NULL,
    namespace_id integer NOT NULL,
    user_id integer NOT NULL,
    foreign key (user_id) references users(id),
    foreign key (namespace_id) references namespaces(id)
);

CREATE TABLE file_attributes (
    id SERIAL PRIMARY KEY,
    file_id integer NOT NULL,
    attribute_id integer NOT NULL,
    foreign key (file_id) references files(id),
    foreign key (attribute_id) references attributes(id)
);
