use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

/// The three kinds of icon lookup, mirroring the fst maps in `libs/iconator`.
/// Stored as the `kind` TEXT column (with a CHECK constraint) rather than a PG
/// enum to keep the Diesel mapping trivial.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum LookupKind {
    Ext,
    Filename,
    Folder,
}

impl LookupKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LookupKind::Ext => "ext",
            LookupKind::Filename => "filename",
            LookupKind::Folder => "folder",
        }
    }
}

impl std::fmt::Display for LookupKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Queryable, Selectable, Debug, Clone, serde::Serialize)]
#[diesel(table_name = crate::schema::icons::lookups)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct IconLookup {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub icon_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::icons::lookups)]
pub struct NewIconLookup {
    pub kind: String,
    pub name: String,
    pub icon_id: i64,
}

impl IconLookup {
    /// Resolve a single `(kind, name)` pair to its icon id, if one exists.
    /// This is the hot path for the API and rides the UNIQUE (kind, name) index.
    pub async fn get_icon_id(
        kind: LookupKind,
        name: &str,
        conn: &mut AsyncPgConnection,
    ) -> Result<Option<i64>, diesel::result::Error> {
        use crate::schema::icons::lookups::dsl;

        dsl::lookups
            .filter(dsl::kind.eq(kind.as_str()))
            .filter(dsl::name.eq(name))
            .select(dsl::icon_id)
            .first::<i64>(conn)
            .await
            .optional()
    }

    /// Bulk insert lookup rows, skipping any that already exist for the same
    /// (kind, name). Used to seed the table from code (parity with the migration
    /// seed and the in-memory loader).
    pub async fn bulk_insert(
        rows: Vec<NewIconLookup>,
        conn: &mut AsyncPgConnection,
    ) -> Result<usize, diesel::result::Error> {
        use crate::schema::icons::lookups::dsl;

        diesel::insert_into(dsl::lookups)
            .values(&rows)
            .on_conflict((dsl::kind, dsl::name))
            .do_nothing()
            .execute(conn)
            .await
    }

    /// Count total rows in the table.
    pub async fn count(
        conn: &mut AsyncPgConnection,
    ) -> Result<i64, diesel::result::Error> {
        use crate::schema::icons::lookups::dsl;

        dsl::lookups.count().get_result(conn).await
    }
}
