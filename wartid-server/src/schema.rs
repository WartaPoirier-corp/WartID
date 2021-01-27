table! {
    sessions (id) {
        id -> Uuid,
        users_id -> Uuid,
        expiration -> Timestamp,
    }
}

table! {
    sessions_oauth2 (token) {
        token -> Varchar,
        users_id -> Uuid,
        user_apps_id -> Uuid,
        initial_scopes -> Varchar,
        expiration -> Timestamp,
    }
}

table! {
    user_apps (id) {
        id -> Uuid,
        name -> Varchar,
        oauth_secret -> Nullable<Varchar>,
        oauth_redirect -> Varchar,
        description -> Nullable<Varchar>,
        hidden -> Bool,
    }
}

table! {
    user_apps_managers (user_apps_id, users_id) {
        user_apps_id -> Uuid,
        users_id -> Uuid,
    }
}

table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        password -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        discord_id -> Nullable<Int8>,
    }
}

joinable!(sessions -> users (users_id));
joinable!(sessions_oauth2 -> user_apps (user_apps_id));
joinable!(sessions_oauth2 -> users (users_id));
joinable!(user_apps_managers -> user_apps (user_apps_id));
joinable!(user_apps_managers -> users (users_id));

allow_tables_to_appear_in_same_query!(
    sessions,
    sessions_oauth2,
    user_apps,
    user_apps_managers,
    users,
);
