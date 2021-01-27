create table sessions(
  id uuid not null primary key default uuid_generate_v4 (),
  users_id uuid not null references users(id) on delete cascade,
  expiration timestamp(0) not null
);

create index idx_sessions_expiration on sessions(expiration);
