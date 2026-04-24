//! Local SQLite database — opening and querying the Osmocom TAC file.

use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Clone)]
pub struct TacRecord {
    pub tac: String,
    pub brand: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gsmarena: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phonearena: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phonedb: Option<String>,
}

/// Metadata about the local database for the `info` command.
pub struct DbInfo {
    pub path: PathBuf,
    pub record_count: i64,
    pub last_updated: Option<u64>,
}

pub struct Database {
    conn: Connection,
    pub path: PathBuf,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA query_only=ON;")?;
        Ok(Self {
            conn,
            path: path.to_path_buf(),
        })
    }

    // ─── Queries ───────────────────────────────────────────────────────────────

    /// Look up a TAC (8 digits). Returns `None` if not found.
    ///
    /// Joins `tac → model → brand` to resolve brand/model names, and pulls
    /// the GSMArena/PhoneArena/PhoneDB slugs from `model` where populated.
    pub fn find_tac(&self, tac: &str) -> Result<Option<TacRecord>> {
        let result = self.conn.query_row(
            "SELECT
                t.tac,
                b.name  AS brand,
                m.name  AS model,
                t.date,
                m.gsmarena,
                m.phonearena,
                m.phonedb
             FROM tac t
             JOIN model m ON m.id    = t.model
             JOIN brand b ON b.id    = m.brand
             WHERE t.tac = ?1",
            params![tac],
            |row| {
                Ok(TacRecord {
                    tac: row.get(0)?,
                    brand: row.get(1)?,
                    model: row.get(2)?,
                    date: row.get(3)?,
                    gsmarena: row.get(4)?,
                    phonearena: row.get(5)?,
                    phonedb: row.get(6)?,
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

    // ─── Info ──────────────────────────────────────────────────────────────────

    pub fn info(&self) -> Result<DbInfo> {
        let last_updated = std::fs::metadata(&self.path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        Ok(DbInfo {
            path: self.path.clone(),
            record_count: self.record_count()?,
            last_updated,
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
