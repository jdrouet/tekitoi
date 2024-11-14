create table providers (
    application_id text not null
        references applications(id) on delete cascade,
    kind int not null,
    primary key (application_id, kind)
);
