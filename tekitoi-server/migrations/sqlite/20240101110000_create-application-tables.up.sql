create table applications (
    id text not null primary key,
    name text unique not null,
    label text,
    client_id text unique not null,
    client_secrets json not null,
    redirect_uri text not null,

    created_at integer not null,
    updated_at integer not null,
    deleted_at integer
);

create table providers (
    id text not null primary key,

    application_id text not null
        references applications(id) on delete cascade,
    name text not null,
    label text,

    config json unique not null,

    created_at integer not null,
    updated_at integer not null,
    deleted_at integer,

    unique (application_id, name)
);
