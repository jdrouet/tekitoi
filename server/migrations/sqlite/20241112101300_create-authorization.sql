create table authorizations (
    code text not null primary key,
    client_id text not null,
    user_id text not null,
    state text not null,
    scope text,
    created_at datetime not null,
    valid_until datetime not null
);
