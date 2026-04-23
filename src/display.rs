//! Terminal output formatting for lookup results and database info.

use colored::Colorize;

use crate::db::DbInfo;
use crate::imei::LookupResult;
use crate::updater::{UpdateOutcome, age_secs};

const SEP: &str = "──────────────────────────────────────────────────";

// ─── Lookup results ────────────────────────────────────────────────────────────

pub fn print_result(result: &LookupResult, no_validate: bool) {
    println!("{}", SEP.dimmed());

    // IMEI line
    println!(
        "  {} {}",
        "IMEI: ".bold(),
        if result.valid {
            result.imei.as_str().green().bold()
        } else {
            result.imei.as_str().red().bold()
        }
    );

    if !result.tac.is_empty() {
        println!("  {} {}", "TAC:  ".bold(), result.tac.cyan());
    }

    // Validation status
    if no_validate && result.is_full_imei {
        println!("  {} {}", "Luhn: ".bold(), "X Validation Skipped".dimmed());
    } else if result.valid {
        if result.imei.len() == 15 {
            println!("  {} {}", "Luhn: ".bold(), "✓ Valid".green());
        }
    } else if let Some(ref err) = result.validation_error {
        println!("  {} {} {}", "Luhn: ".bold(), "⚠ ".yellow(), err.yellow());
    }

    // Device match
    match &result.device {
        Some(rec) => {
            println!("  {} {}", "Brand:".bold(), rec.brand.bold());
            println!("  {} {}", "Model:".bold(), rec.model);
        }
        None if !result.tac.is_empty() => {
            println!(
                "  {} {}",
                "Match:".bold(),
                "TAC not found in local database".yellow()
            );
        }
        None => {}
    }
}

pub fn print_results_footer() {
    println!("{}", SEP.dimmed());
}

// ─── Update outcome ────────────────────────────────────────────────────────────

pub fn print_update_outcome(outcome: &UpdateOutcome, db: &crate::db::Database) {
    if !outcome.ran {
        if let Ok(Some(age)) = age_secs(db) {
            let age_days = age / 86400;
            println!(
                "{} Database is {} day(s) old — use {} to force a re-download.",
                "✓".green().bold(),
                age_days,
                "--force".yellow(),
            );
        }
        return;
    }

    println!(
        "{} Imported {} TAC records.",
        "✓".green().bold(),
        outcome.records_imported.to_string().bold()
    );
    println!("  Saved to: {}", db.path.display().to_string().dimmed());

    if !outcome.parse_errors.is_empty() {
        println!(
            "  {} {} row(s) skipped due to parse errors.",
            "⚠".yellow(),
            outcome.parse_errors.len()
        );
    }
}

pub fn print_update_start() {
    println!("{} Downloading Osmocom TAC database…", "→".cyan().bold());
    println!("  (this may take a moment on the first run)");
}

// ─── Database info ─────────────────────────────────────────────────────────────

pub fn print_db_info(info: &DbInfo) {
    println!("{}", "tac-lookup — Database Info".bold().underline());
    println!("  Path:    {}", info.path.display().to_string().cyan());
    println!("  Records: {}", info.record_count.to_string().bold());

    match info.last_updated {
        Some(ts) => {
            let age = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(ts);

            let age_str = if age < 3600 {
                format!("{} minute(s) ago", age / 60)
            } else if age < 86400 {
                format!("{} hour(s) ago", age / 3600)
            } else {
                format!("{} day(s) ago", age / 86400)
            };

            println!("  Updated: {}", age_str.bold());
        }
        None => println!("  Updated: {}", "never".dimmed()),
    }
}

pub fn print_db_missing(path: &std::path::Path) {
    println!("  Path:   {}", path.display().to_string().cyan());
    println!(
        "  Status: {} — run {} to download it.",
        "not found".red().bold(),
        "tac-lookup update".yellow()
    );
}
