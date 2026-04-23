use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

use tac_lookup::{db, display, imei, updater};

#[derive(Parser)]
#[command(
    name = "tac-lookup",
    about = "Fast IMEI/TAC lookup using a local Osmocom TAC database",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Look up device info for one or more IMEI numbers (or 8-digit TAC codes)
    #[command(alias = "lookup")]
    Check {
        /// 15-digit IMEI numbers or 8-digit TAC codes to look up
        #[arg(required = true, num_args = 1..)]
        imeis: Vec<String>,

        /// Output raw JSON instead of pretty-printed text
        #[arg(short, long)]
        json: bool,

        /// Skip Luhn validation
        #[arg(long)]
        no_validate: bool,
    },

    /// Download or refresh the local Osmocom TAC database
    Update {
        /// Force database re-download
        #[arg(short, long)]
        force: bool,
    },

    /// Show database path, record count, and last update time
    Info,
}

// ─── Command handlers ──────────────────────────────────────────────────────────

fn run_check(imeis: Vec<String>, json: bool, no_validate: bool) -> Result<()> {
    let path = db::default_db_path()?;

    if !path.exists() {
        eprintln!(
            "{} Local database not found. Run {} first.",
            "✗".red().bold(),
            "tac-lookup update".yellow()
        );
        std::process::exit(1);
    }

    let database = db::Database::open(&path)?;

    let results: Vec<imei::LookupResult> = imeis
        .iter()
        .map(|raw| imei::lookup(raw.trim(), &database, no_validate))
        .collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for result in &results {
            display::print_result(result, no_validate);
        }
        display::print_results_footer();
    }

    Ok(())
}

fn run_update(force: bool) -> Result<()> {
    let path = db::default_db_path()?;
    let database = db::Database::open(&path)?;

    display::print_update_start();
    let outcome = updater::run(&database, force)?;
    display::print_update_outcome(&outcome, &database);

    Ok(())
}

fn run_info() -> Result<()> {
    let path = db::default_db_path()?;

    if !path.exists() {
        println!("{}", "tac-lookup — Database Info".bold().underline());
        display::print_db_missing(&path);
        return Ok(());
    }

    let database = db::Database::open(&path)?;
    let info = database.info()?;
    display::print_db_info(&info);

    Ok(())
}

// ─── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Check {
            imeis,
            json,
            no_validate,
        } => run_check(imeis, json, no_validate),
        Commands::Update { force } => run_update(force),
        Commands::Info => run_info(),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}
