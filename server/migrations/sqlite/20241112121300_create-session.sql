create table sessions (
    access_token text not null primary key,
    client_id text not null references applications(id) on delete cascade,
    user_id text not null references users(id) on delete cascade,
    scope text,
    created_at datetime not null,
    valid_until datetime not null
);
