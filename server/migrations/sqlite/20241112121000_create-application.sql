create table applications (
    id text not null primary key,
    secrets text not null,
    redirect_uri text not null
);
