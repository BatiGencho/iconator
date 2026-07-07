// @generated automatically by Diesel CLI.

pub mod icons {
    diesel::table! {
        icons.lookups (id) {
            id -> Uuid,
            kind -> Text,
            name -> Text,
            icon_id -> Int8,
            created_at -> Timestamptz,
        }
    }

    diesel::table! {
        icons.query_history (id) {
            id -> Uuid,
            query_kind -> Text,
            query_path -> Text,
            icon_id -> Nullable<Int8>,
            created_at -> Timestamptz,
        }
    }

    diesel::allow_tables_to_appear_in_same_query!(lookups, query_history,);
}
