create table user_apps (
    id uuid primary key default uuid_generate_v4 (),
    name varchar not null,
    oauth_secret varchar(64) default null,
    description varchar default null,
    hidden boolean not null default false
);

create table user_apps_managers (
    user_app uuid primary key references user_apps(id) on delete cascade,
    account uuid references users(id) on delete cascade
);
