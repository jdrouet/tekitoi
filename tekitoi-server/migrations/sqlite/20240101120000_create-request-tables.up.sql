create table application_authorization_requests (
    id text not null primary key,
    application_id text not null
        references applications(id) on delete cascade,

    code_challenge text not null,
    code_challenge_method text not null,
    state text not null,
    redirect_uri text not null,

    created_at integer not null,
    expired_at integer not null,
    deleted_at integer
);

create table provider_authorization_requests (
    id text not null primary key,

    application_authorization_request_id text not null
        references application_authorization_requests(id) on delete cascade,
    provider_id text not null
        references providers(id) on delete cascade,

    csrf_token text not null,
    pkce_verifier text not null,

    created_at integer not null,
    expired_at integer not null,
    deleted_at integer,

    unique (application_authorization_request_id, csrf_token)
);

create table redirect_requests (
    id text not null primary key,

    provider_authorization_request_id text not null
        references provider_authorization_requests(id) on delete cascade,

    code text not null,

    created_at integer not null,
    expired_at integer not null,
    deleted_at integer
);

create table access_tokens (
    id text not null primary key,
    redirect_request_id text not null
        references redirect_requests(id) on delete cascade,
    token text not null,
    created_at integer not null,
    expired_at integer,
    deleted_at integer
)