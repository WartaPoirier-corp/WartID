create table sessions(
  id uuid not null primary key default uuid_generate_v4 (),
  account uuid not null,
  expiration timestamp(0) not null,
  foreign key (account) references users(id) on delete cascade
);

create index idx_sessions_expiration on sessions(expiration);
