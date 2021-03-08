CREATE TABLE namespaces (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    creator integer NOT NULL,
    foreign key (creator) references users(id)
);
