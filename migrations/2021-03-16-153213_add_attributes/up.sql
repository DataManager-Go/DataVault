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
