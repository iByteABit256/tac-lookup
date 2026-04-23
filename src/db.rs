//! Local SQLite database — schema, queries, and metadata.

use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A single TAC record as stored in the local database.
#[derive(Debug, Serialize, Clone)]
pub struct TacRecord {
    pub tac: String,
    pub brand: String,
    pub model: String,
}

/// Metadata about the local database for the `info` command.
pub struct DbInfo {
    pub path: PathBuf,
    pub record_count: i64,
    /// Unix timestamp of the last successful import, if any.
    pub last_updated: Option<u64>,
}

pub struct Database {
    conn: Connection,
    pub path: PathBuf,
}

impl Database {
    /// Open (or create) the database at the given path, applying PRAGMAs and schema.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;",
        )?;
        let db = Self {
            conn,
            path: path.to_path_buf(),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tac (
                tac   TEXT PRIMARY KEY,
                brand TEXT NOT NULL,
                model TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
             );",
        )?;
        Ok(())
    }

    // ─── Queries ───────────────────────────────────────────────────────────────

    /// Look up a TAC (8 digits). Returns `None` if not found.
    pub fn find_tac(&self, tac: &str) -> Result<Option<TacRecord>> {
        let result = self.conn.query_row(
            "SELECT tac, brand, model FROM tac WHERE tac = ?1",
            params![tac],
            |row| {
                Ok(TacRecord {
                    tac: row.get(0)?,
                    brand: row.get(1)?,
                    model: row.get(2)?,
                })
            },
        );
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn record_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM tac", [], |r| r.get(0))?)
    }

    pub fn last_updated(&self) -> Result<Option<u64>> {
        let result: rusqlite::Result<String> =
            self.conn
                .query_row("SELECT value FROM meta WHERE key='updated_at'", [], |r| {
                    r.get(0)
                });
        match result {
            Ok(v) => Ok(v.parse::<u64>().ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ─── Bulk import ───────────────────────────────────────────────────────────

    /// Delete all existing TAC rows and insert `rows` in a single transaction.
    /// Returns the number of rows successfully inserted.
    pub fn replace_all(&self, rows: &[(String, String, String)]) -> Result<usize> {
        self.conn.execute_batch("DELETE FROM tac;")?;

        let mut stmt = self
            .conn
            .prepare("INSERT OR REPLACE INTO tac (tac, brand, model) VALUES (?1, ?2, ?3)")?;

        let mut count = 0usize;
        for (tac, brand, model) in rows {
            stmt.execute(params![tac, brand, model])?;
            count += 1;
        }
        Ok(count)
    }

    /// Record the current time as the last-updated timestamp.
    pub fn touch_updated_at(&self) -> Result<()> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('updated_at', ?1)",
            params![ts.to_string()],
        )?;
        Ok(())
    }

    // ─── Info ──────────────────────────────────────────────────────────────────

    pub fn info(&self) -> Result<DbInfo> {
        Ok(DbInfo {
            path: self.path.clone(),
            record_count: self.record_count()?,
            last_updated: self.last_updated()?,
        })
    }
}

const DB_FILE: &str = "tacdb.sqlite3";

pub fn default_db_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "tac-lookup", "tac-lookup")
        .ok_or_else(|| anyhow!("Could not determine a cache directory for this platform"))?;
    let cache = dirs.cache_dir().to_path_buf();
    std::fs::create_dir_all(&cache)?;
    Ok(cache.join(DB_FILE))
}
