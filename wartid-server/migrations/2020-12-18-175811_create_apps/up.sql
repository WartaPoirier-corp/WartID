create table user_apps (
    id uuid primary key default uuid_generate_v4 (),
    name varchar not null,
    oauth_secret varchar(64) default null,
    oauth_redirect varchar not null default '',
    description varchar default null,
    hidden boolean not null default false
);

create table user_apps_managers (
    user_apps_id uuid not null references user_apps(id) on delete cascade,
    users_id uuid not null references users(id) on delete cascade,

    primary key (user_apps_id, users_id)
);
