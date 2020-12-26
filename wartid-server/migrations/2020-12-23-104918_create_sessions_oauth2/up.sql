create table sessions_oauth2 (
    token varchar(32) not null primary key,

    users_id uuid not null references users(id) on delete cascade,
    user_apps_id uuid not null references user_apps(id) on delete cascade,
    initial_scopes varchar not null,
    expiration timestamp(0) not null,

    unique (users_id, user_apps_id)
);
