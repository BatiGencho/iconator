use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

#[derive(Queryable, Selectable, Debug, Clone, serde::Serialize)]
#[diesel(table_name = crate::schema::icons::query_history)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct QueryHistory {
    pub id: Uuid,
    /// 'file' or 'folder' - which lookup endpoint served the request.
    pub query_kind: String,
    /// The raw path/name the caller asked about.
    pub query_path: String,
    /// Resolved icon id, or NULL when no icon matched.
    pub icon_id: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::icons::query_history)]
pub struct NewQueryHistory {
    pub query_kind: String,
    pub query_path: String,
    pub icon_id: Option<i64>,
}

impl QueryHistory {
    pub async fn create(
        entry: NewQueryHistory,
        conn: &mut AsyncPgConnection,
    ) -> Result<Self, diesel::result::Error> {
        use crate::schema::icons::query_history::dsl::*;

        diesel::insert_into(query_history)
            .values(&entry)
            .returning(QueryHistory::as_returning())
            .get_result(conn)
            .await
    }

    /// Get the last N query history entries ordered by most recent first.
    pub async fn get_latest(
        limit: i64,
        conn: &mut AsyncPgConnection,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        use crate::schema::icons::query_history::dsl::*;

        query_history
            .order(created_at.desc())
            .limit(limit)
            .load(conn)
            .await
    }
}
