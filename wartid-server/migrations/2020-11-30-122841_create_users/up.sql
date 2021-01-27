create extension if not exists "uuid-ossp";

create table users (
    id uuid primary key default uuid_generate_v4 (),
    username varchar(64) not null unique,
    password varchar default null,
    email varchar default null,
    discord_id bigint default null unique
);

create unique index idx_users_username on users(username);
create unique index idx_users_discord on users(discord_id);
