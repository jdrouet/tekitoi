create table authorizations (
    code text not null primary key,
    client_id text not null references applications(id) on delete cascade,
    user_id text not null references users(id) on delete cascade,
    state text not null,
    scope text,

    code_challenge text not null,
    code_challenge_method text not null,
    response_type text not null,

    created_at datetime not null,
    valid_until datetime not null
);
