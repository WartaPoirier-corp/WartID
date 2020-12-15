table! {
    sessions (id) {
        id -> Uuid,
        account -> Uuid,
        expiration -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Uuid,
        username -> Nullable<Varchar>,
        password -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        discord_id -> Nullable<Int8>,
    }
}

joinable!(sessions -> users (account));

allow_tables_to_appear_in_same_query!(
    sessions,
    users,
);
