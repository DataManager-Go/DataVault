CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    uploader integer NOT NULL,
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
    foreign key (uploader) references users(id)
);
ALTER SEQUENCE files_id_seq RESTART WITH 1;
