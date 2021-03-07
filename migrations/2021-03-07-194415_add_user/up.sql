CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username text NOT NULL,
    password text NOT NULL,
    disabled boolean NOT NULL DEFAULT False,
    UNIQUE(username)
);
