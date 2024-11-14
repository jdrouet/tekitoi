create table users (
    id text not null primary key,
    application_id text not null
        references applications(id) on delete cascade,
    provider_kind tinyint not null,
    login text not null,
    email text not null
);
