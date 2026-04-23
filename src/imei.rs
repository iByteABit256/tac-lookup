//! IMEI and TAC parsing, validation, and core data types.

use anyhow::{Result, anyhow};
use serde::Serialize;

// ─── Types ─────────────────────────────────────────────────────────────────────

/// A parsed and validated IMEI or TAC input.
#[derive(Debug)]
pub struct ParsedImei {
    /// The 8-digit Type Allocation Code (first 8 digits of the IMEI).
    pub tac: String,
    /// The full normalized digit string — either 15 digits (IMEI) or 8 (TAC-only).
    pub normalized: String,
    /// Whether this was a full 15-digit IMEI (as opposed to a bare TAC).
    pub is_full_imei: bool,
}

/// Result of a single IMEI/TAC lookup, ready for display or serialisation.
#[derive(Debug, Serialize)]
pub struct LookupResult {
    pub imei: String,
    pub tac: String,
    /// `true` if the input was a bare TAC, Luhn passed, or validation was skipped.
    pub valid: bool,
    #[serde(skip_serializing)]
    pub is_full_imei: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<crate::db::TacRecord>,
}

// ─── Parsing ───────────────────────────────────────────────────────────────────

/// Strip non-digit characters and return a `ParsedImei`, or an error if the
/// length is not 8 (bare TAC) or 15 (full IMEI).
pub fn parse(input: &str) -> Result<ParsedImei> {
    let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    match digits.len() {
        8 => Ok(ParsedImei {
            tac: digits.clone(),
            normalized: digits,
            is_full_imei: false,
        }),
        15 => Ok(ParsedImei {
            tac: digits[..8].to_string(),
            normalized: digits,
            is_full_imei: true,
        }),
        n => Err(anyhow!(
            "Expected a 15-digit IMEI or 8-digit TAC, got {} digits",
            n
        )),
    }
}

// ─── Luhn validation ───────────────────────────────────────────────────────────

/// Returns `true` if `imei` (exactly 15 digits) passes the Luhn check.
pub fn luhn_valid(imei: &str) -> bool {
    let digits: Vec<u32> = imei.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 15 {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let doubled = d * 2;
                if doubled > 9 { doubled - 9 } else { doubled }
            } else {
                d
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

// ─── High-level lookup ─────────────────────────────────────────────────────────

/// Parse `input`, optionally validate via Luhn, and query the local database.
/// Always returns a `LookupResult` — parse/validation errors are embedded in it
/// rather than propagated, so a batch of IMEIs can accumulate results gracefully.
pub fn lookup(input: &str, db: &crate::db::Database, skip_luhn: bool) -> LookupResult {
    let parsed = match parse(input) {
        Ok(p) => p,
        Err(e) => {
            return LookupResult {
                imei: input.to_string(),
                tac: String::new(),
                valid: false,
                is_full_imei: true,
                validation_error: Some(e.to_string()),
                device: None,
            };
        }
    };

    let (valid, validation_error) = if parsed.is_full_imei && !skip_luhn {
        if luhn_valid(&parsed.normalized) {
            (true, None)
        } else {
            (
                false,
                Some("Luhn check failed — IMEI may be invalid".to_string()),
            )
        }
    } else {
        (true, None)
    };

    let device = db.find_tac(&parsed.tac).unwrap_or_else(|e| {
        eprintln!("db error: {}", e);
        None
    });

    LookupResult {
        imei: parsed.normalized,
        tac: parsed.tac,
        valid,
        is_full_imei: parsed.is_full_imei,
        validation_error,
        device,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_imei() {
        let p = parse("490154203237518").unwrap();
        assert_eq!(p.tac, "49015420");
        assert!(p.is_full_imei);
    }

    #[test]
    fn parse_tac_only() {
        let p = parse("49015420").unwrap();
        assert_eq!(p.tac, "49015420");
        assert!(!p.is_full_imei);
    }

    #[test]
    fn parse_strips_separators() {
        let p = parse("490154-2032-37518").unwrap();
        assert_eq!(p.normalized, "490154203237518");
    }

    #[test]
    fn parse_bad_length() {
        assert!(parse("12345").is_err());
    }

    #[test]
    fn luhn_known_valid() {
        // 490154203237518 is the canonical Luhn-valid test IMEI
        assert!(luhn_valid("490154203237518"));
    }

    #[test]
    fn luhn_known_invalid() {
        assert!(!luhn_valid("490154203237519"));
    }
}
