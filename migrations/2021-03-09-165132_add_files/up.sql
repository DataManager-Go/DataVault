CREATE TABLE public.files (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    uploader integer NOT NULL,
    local_name text NOT NULL,
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    file_size bigint NOT NULL,
    file_type text NOT NULL,
    is_public boolean DEFAULT false,
    public_filename text,
    namespace_id integer NOT NULL,
    encryption integer NOT NULL DEFAULT 0,
    checksum text NOT NULL,
    foreign key (uploader) references users(id)
);

