//! Core data models for SPL transfer building.
//!
//! Every field in every input is treated as strictly-typed structured data,
//! never as natural language that could be "reinterpreted". This is the
//! structural defence against prompt injection.

use serde::{Deserialize, Serialize};

/// Request to build an unsigned transfer transaction.
///
/// All fields are strictly validated:
/// - `from` / `to`: must be valid base58 Solana pubkeys
/// - `amount`: must be a positive integer string (never "all", "max", etc.)
/// - `mint`: if present, must be a valid base58 pubkey
/// - `memo`: treated as opaque text data, never parsed as instructions
#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    /// Sender pubkey (also fee payer).
    pub from: String,
    /// Recipient pubkey.
    pub to: String,
    /// Amount in smallest units (lamports for SOL, raw amount for SPL).
    /// MUST be a positive integer string. Words like "all", "max", "100%"
    /// are rejected.
    pub amount: String,
    /// Token mint address. None = native SOL transfer.
    pub mint: Option<String>,
    /// Optional memo text for invoice reconciliation.
    pub memo: Option<String>,
}

/// Result of building an unsigned transaction.
#[derive(Debug, Serialize)]
pub struct BuildResult {
    /// Base64-encoded unsigned transaction bytes.
    pub unsigned_tx_base64: String,
    /// Human-readable summary for approval gate (Telegram/Discord).
    pub human_summary: String,
    /// Whether a new ATA will be created for the recipient.
    pub will_create_ata: bool,
    /// Estimated transaction fee in lamports (base fee + rent if ATA created).
    pub estimated_fee_lamports: u64,
}
