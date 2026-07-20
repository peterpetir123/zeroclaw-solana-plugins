//! RPC transport abstraction trait.
//!
//! The core logic never calls HTTP directly. It receives `&dyn SolanaRpc`.
//! - In native tests: `MockRpc` (see each plugin's `rpc_mock` module).
//! - In wasm: `WakiRpc` (implemented in each plugin's component module).

use serde::{Deserialize, Serialize};

/// Minimal account info returned by `getAccountInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Base64-encoded account data (from `encoding: "base64"`).
    pub data_base64: String,
    /// Owner program ID (base58).
    pub owner: String,
    /// Lamport balance.
    pub lamports: u64,
    /// Whether this account is executable.
    pub executable: bool,
}

/// Transport trait for Solana JSON-RPC calls.
///
/// Implementations must handle JSON-RPC envelope wrapping/unwrapping.
/// All methods return `Err(String)` on transport or parsing errors.
pub trait SolanaRpc {
    /// Fetch account info for a given pubkey (base58).
    /// Returns `None` if the account does not exist on-chain.
    fn get_account_info(&self, pubkey: &str) -> Result<Option<AccountInfo>, String>;

    /// Fetch the latest blockhash as a base58 string.
    fn get_latest_blockhash(&self) -> Result<String, String>;

    /// Fetch the minimum balance in lamports for rent exemption of an account
    /// with the given data size in bytes.
    fn get_minimum_balance_for_rent_exemption(&self, size: u64) -> Result<u64, String>;
}
