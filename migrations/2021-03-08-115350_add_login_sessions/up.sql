CREATE TABLE login_sessions (
    id SERIAL PRIMARY KEY,
    user_id integer NOT NULL,
    token text NOT NULL,
    requests bigint NOT NULL DEFAULT 0,
    machine_id text,
    UNIQUE(token),
    foreign key (user_id) references users(id)
);
