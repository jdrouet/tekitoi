create table users (
    id text not null primary key,
    application_id text not null
        references applications(id) on delete cascade,
    login text not null,
    email text not null
);
