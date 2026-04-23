//! Downloads the Osmocom TAC CSV and imports it into the local database.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::db::Database;

const OSMOCOM_CSV_URL: &str = "http://tacdb.osmocom.org/export/tacdb.csv";
/// Minimum age in seconds before `update` considers the database stale (7 days).
const STALE_AFTER_SECS: u64 = 7 * 24 * 60 * 60;

// ─── Osmocom CSV row ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OsmocomRow {
    tac: String,
    brand: String,
    model: String,
}

// ─── Staleness check ───────────────────────────────────────────────────────────

/// How many seconds old the database is, or `None` if it has never been updated.
pub fn age_secs(db: &Database) -> Result<Option<u64>> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(db.last_updated()?.map(|ts| now.saturating_sub(ts)))
}

/// Returns `true` if the database should be (re-)downloaded.
pub fn is_stale(db: &Database) -> Result<bool> {
    match age_secs(db)? {
        None => Ok(true), // never downloaded
        Some(age) => Ok(age >= STALE_AFTER_SECS),
    }
}

// ─── Download ──────────────────────────────────────────────────────────────────

/// Download the Osmocom CSV and return its body as a `String`.
pub fn download_csv() -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION"),
        ))
        .build()?;

    let resp = client
        .get(OSMOCOM_CSV_URL)
        .send()
        .context("Failed to reach tacdb.osmocom.org")?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "Server returned HTTP {} for {}",
            resp.status(),
            OSMOCOM_CSV_URL
        ));
    }

    resp.text().context("Failed to read response body")
}

// ─── Parse ─────────────────────────────────────────────────────────────────────

/// Parse the Osmocom CSV body and return `(tac, brand, model)` tuples.
/// Rows with a non-8-digit TAC are silently skipped; other parse errors are
/// returned as a collected `Vec` of error strings alongside the good rows.
pub fn parse_csv(body: &str) -> (Vec<(String, String, String)>, Vec<String>) {
    let mut rows = Vec::new();
    let mut errors = Vec::new();

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(body.as_bytes());

    for result in rdr.deserialize::<OsmocomRow>() {
        match result {
            Ok(row) => {
                let tac = row.tac.trim().to_string();
                if tac.len() == 8 && tac.chars().all(|c| c.is_ascii_digit()) {
                    rows.push((
                        tac,
                        row.brand.trim().to_string(),
                        row.model.trim().to_string(),
                    ));
                }
                // silently skip TACs that don't look like 8-digit codes
            }
            Err(e) => errors.push(e.to_string()),
        }
    }

    (rows, errors)
}

// ─── High-level update ─────────────────────────────────────────────────────────

/// The outcome of an `update` operation.
pub struct UpdateOutcome {
    pub records_imported: usize,
    pub parse_errors: Vec<String>,
    /// `true` if the download+import actually ran; `false` if skipped as fresh.
    pub ran: bool,
}

/// Download and import the Osmocom database.
///
/// If `force` is `false` and the database is less than 7 days old, the
/// operation is skipped and `UpdateOutcome::ran` is `false`.
pub fn run(db: &Database, force: bool) -> Result<UpdateOutcome> {
    if !force && !is_stale(db)? {
        return Ok(UpdateOutcome {
            records_imported: 0,
            parse_errors: vec![],
            ran: false,
        });
    }

    let body = download_csv()?;
    let (rows, parse_errors) = parse_csv(&body);
    let records_imported = db.replace_all(&rows)?;
    db.touch_updated_at()?;

    Ok(UpdateOutcome {
        records_imported,
        parse_errors,
        ran: true,
    })
}
