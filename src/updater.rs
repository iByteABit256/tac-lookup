//! Downloads the Osmocom TAC SQLite file to the local cache.

use anyhow::{Context, Result, anyhow};
use std::io::Write;
use std::path::Path;

use crate::db::Database;

const OSMOCOM_SQLITE_URL: &str = "http://tacdb.osmocom.org/export/tacdb.sqlite3";
/// Minimum age in seconds before `update` considers the database stale (7 days).
const STALE_AFTER_SECS: u64 = 7 * 24 * 60 * 60;

// ─── Staleness check ───────────────────────────────────────────────────────────

/// How many seconds old the database file is based on its mtime, or `None` if
/// the file doesn't exist yet.
pub fn age_secs(path: &Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let age = std::time::SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default()
        .as_secs();
    Some(age)
}

/// Returns `true` if the database file should be (re-)downloaded.
pub fn is_stale(path: &Path) -> bool {
    match age_secs(path) {
        None => true, // file doesn't exist yet
        Some(age) => age >= STALE_AFTER_SECS,
    }
}

// ─── Download ──────────────────────────────────────────────────────────────────

/// Download the Osmocom SQLite file, writing it atomically to `dest`.
///
/// We download to a sibling `.tmp` file first and rename on success, so a
/// failed or interrupted download never leaves a corrupt database behind.
pub fn download_sqlite(dest: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION"),
        ))
        .build()?;

    let resp = client
        .get(OSMOCOM_SQLITE_URL)
        .send()
        .context("Failed to reach tacdb.osmocom.org")?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "Server returned HTTP {} for {}",
            resp.status(),
            OSMOCOM_SQLITE_URL
        ));
    }

    let bytes = resp.bytes().context("Failed to read response body")?;

    // Write to a temp file next to the destination, then rename atomically.
    let tmp = dest.with_extension("sqlite3.tmp");
    let mut file = std::fs::File::create(&tmp)
        .with_context(|| format!("Could not create temp file: {}", tmp.display()))?;
    file.write_all(&bytes)
        .context("Failed to write database to disk")?;
    std::fs::rename(&tmp, dest).context("Failed to move downloaded database into place")?;

    Ok(())
}

// ─── High-level update ─────────────────────────────────────────────────────────

/// The outcome of an `update` operation.
pub struct UpdateOutcome {
    pub record_count: i64,
    /// `true` if the download actually ran; `false` if skipped as fresh.
    pub ran: bool,
}

/// Download the Osmocom SQLite database to the cache path.
///
/// If `force` is `false` and the file is less than 7 days old, the operation
/// is skipped and `UpdateOutcome::ran` is `false`.
pub fn run(db: &Database, force: bool) -> Result<UpdateOutcome> {
    if !force && !is_stale(&db.path) {
        return Ok(UpdateOutcome {
            record_count: db.record_count()?,
            ran: false,
        });
    }

    download_sqlite(&db.path)?;

    // Re-open to count records in the freshly downloaded file.
    let fresh = Database::open(&db.path)?;
    Ok(UpdateOutcome {
        record_count: fresh.record_count()?,
        ran: true,
    })
}
