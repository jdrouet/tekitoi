create table applications (
    id text not null primary key,
    name text unique not null,
    label text,
    client_id text unique not null,
    redirect_uri text not null,

    created_at integer not null,
    updated_at integer not null,
    deleted_at integer
);

create table application_client_secrets (
    application_id text not null
        references applications(id) on delete cascade,
    content text not null,

    created_at integer not null,
    deleted_at integer,

    primary key (application_id, content)
);

create table providers (
    id text not null primary key,

    application_id text not null
        references applications(id) on delete cascade,
    kind text not null,
    name text not null,
    label text,

    client_id text unique not null,
    client_secret text not null,
    authorization_url text not null,
    token_url text not null,
    base_api_url text not null,

    created_at integer not null,
    updated_at integer not null,
    deleted_at integer,

    unique (application_id, name)
);

create table provider_scopes (
    provider_id text not null
        references providers(id) on delete cascade,
    content text not null,

    primary key (provider_id, content)
);