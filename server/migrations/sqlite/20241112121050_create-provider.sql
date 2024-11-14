create table providers (
    application_id text not null
        references applications(id) on delete cascade,
    kind tinyint not null,
    primary key (application_id, kind)
);
