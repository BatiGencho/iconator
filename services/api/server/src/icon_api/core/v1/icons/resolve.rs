//! Path -> icon resolution, shared by the DB and in-memory handlers. Files match
//! on their exact name first, then their extension; folders match on their exact
//! name. These are the same rules `iconator` uses, so both backends always agree.

use std::path::Path;

use diesel_async::AsyncPgConnection;
use postgres::models::icon_lookup::{IconLookup, LookupKind};

/// Which lookup endpoint is being served.
#[derive(Debug, Clone, Copy)]
pub enum Target {
    File,
    Folder,
}

impl Target {
    /// Value stored in `query_history.query_kind`.
    pub fn as_str(self) -> &'static str {
        match self {
            Target::File => "file",
            Target::Folder => "folder",
        }
    }
}

/// Resolve a path against Postgres, mirroring `iconator`'s matching order.
pub async fn resolve_db(
    target: Target,
    path: &str,
    conn: &mut AsyncPgConnection,
) -> Result<Option<i64>, diesel::result::Error> {
    let path = Path::new(path);
    let Some(basename) = path.file_name().and_then(|s| s.to_str()) else {
        return Ok(None);
    };

    match target {
        Target::Folder => {
            IconLookup::get_icon_id(LookupKind::Folder, basename, conn).await
        }
        Target::File => {
            // Exact file name first (e.g. "CMakeCache.txt").
            if let Some(id) =
                IconLookup::get_icon_id(LookupKind::Filename, basename, conn)
                    .await?
            {
                return Ok(Some(id));
            }
            // Then fall back to the extension (e.g. "rs").
            match path.extension().and_then(|s| s.to_str()) {
                Some(ext) => {
                    IconLookup::get_icon_id(LookupKind::Ext, ext, conn).await
                }
                None => Ok(None),
            }
        }
    }
}

/// Resolve a path against the in-memory `iconator` fst maps.
pub fn resolve_memory(target: Target, path: &str) -> Option<i64> {
    let id = match target {
        Target::File => iconator::get_icon_for_file(path),
        Target::Folder => iconator::get_icon_for_folder(path),
    };
    id.map(|id| id as i64)
}
