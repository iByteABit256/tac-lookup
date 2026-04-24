# tac-lookup

A fast, offline-first CLI for looking up device information from an IMEI or TAC code. No API keys, no rate limits, no subscriptions — just a local copy of the community-maintained [Osmocom TAC database](http://tacdb.osmocom.org/).

## How it works

The first 8 digits of any IMEI form the **TAC (Type Allocation Code)**, which identifies the device manufacturer and model. `tac-lookup` downloads the Osmocom TAC database once and stores it locally as SQLite, so every subsequent lookup is instant and fully offline.

> **Note:** Blacklist status, carrier lock, and iCloud status live in private carrier/GSMA systems. No free public source exists for those — any service offering them is a paid reseller.

## Install

Requires Rust — install via [rustup](https://rustup.rs) if needed.

```bash
git clone https://github.com/iByteABit256/tac-lookup
cd tac-lookup
cargo build --release
```

The binary will be at `./target/release/tac-lookup`. To install it globally:

```bash
cargo install --path .
```

## Usage

**First run — download the database:**

```bash
tac-lookup update
```

This fetches the Osmocom SQLite file and stores in your platform's cache directory:

- **Linux:** `~/.cache/tac-lookup/tacdb.sqlite3`
- **macOS:** `~/Library/Caches/com.tac-lookup.tac-lookup/tacdb.sqlite3`
- **Windows:** `%LOCALAPPDATA%\tac-lookup\tac-lookup\cache\tacdb.sqlite3`

Subsequent `tac-lookup update` calls are no-ops if the database is less than 7 days old. Use `--force` to always re-download.

**Look up an IMEI or TAC:**

```bash
# Single IMEI
tac-lookup check 352399110123456

# Multiple IMEIs at once
tac-lookup check 352399110123456 490154203237518 013368001234567

# 8-digit TAC directly (skips Luhn validation)
tac-lookup check 35239911

# JSON output — pipe-friendly with jq
tac-lookup check 352399110123456 --json | jq .

# Skip Luhn validation (useful for synthetic or partial codes)
tac-lookup check 352399110123456 --no-validate
```

**Database info:**

```bash
tac-lookup info
```

## Example output

```
──────────────────────────────────────────────────
  IMEI:  355394074211242
  TAC:   35539407
  Luhn:  ✓ Valid
  Brand: Apple
  Model: iPhone 6
  Date:  2017-10-05 08:51:46
  PA:    http://www.phonearena.com/phones/Apple-iPhone-6_id8346
──────────────────────────────────────────────────
```

## Data source

Device data comes from the [Osmocom TAC Database](http://tacdb.osmocom.org/), a community-maintained, publicly downloadable registry of Type Allocation Codes, licensed under [CC-BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/).

## License

GPL-3.0 — see [LICENSE](LICENSE).
