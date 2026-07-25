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

/// Pure JSON-RPC response parser for `getAccountInfo`
pub fn parse_get_account_info_response(result: &serde_json::Value) -> Result<Option<AccountInfo>, String> {
    let value = result.get("value");
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(val) => {
            let data_arr = val.get("data")
                .and_then(|d| d.as_array())
                .ok_or("missing data array in account info")?;
            let data_base64 = data_arr.first()
                .and_then(|v| v.as_str())
                .ok_or("missing base64 data")?
                .to_string();
            let owner = val.get("owner")
                .and_then(|v| v.as_str())
                .ok_or("missing owner")?
                .to_string();
            let lamports = val.get("lamports")
                .and_then(|v| v.as_u64())
                .ok_or("missing lamports")?;
            let executable = val.get("executable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            Ok(Some(AccountInfo {
                data_base64,
                owner,
                lamports,
                executable,
            }))
        }
    }
}

/// Pure JSON-RPC response parser for `getLatestBlockhash`
pub fn parse_get_latest_blockhash_response(result: &serde_json::Value) -> Result<String, String> {
    result.get("value")
        .and_then(|v| v.get("blockhash"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "failed to parse blockhash from response".to_string())
}

/// Pure JSON-RPC response parser for `getMinimumBalanceForRentExemption`
pub fn parse_get_minimum_balance_for_rent_exemption_response(result: &serde_json::Value) -> Result<u64, String> {
    result.as_u64()
        .ok_or_else(|| "failed to parse rent exemption amount".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_get_account_info_json_rpc_response() {
        let raw = serde_json::json!({
            "value": {
                "data": ["base64data==", "base64"],
                "owner": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                "lamports": 1461600,
                "executable": false
            }
        });
        let info = parse_get_account_info_response(&raw).unwrap().unwrap();
        assert_eq!(info.owner, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        assert_eq!(info.data_base64, "base64data==");
        assert_eq!(info.lamports, 1461600);
        assert!(!info.executable);
    }

    #[test]
    fn parses_null_value_as_account_not_found() {
        let raw = serde_json::json!({ "value": null });
        assert!(parse_get_account_info_response(&raw).unwrap().is_none());
    }

    #[test]
    fn parses_latest_blockhash_response() {
        let raw = serde_json::json!({
            "value": {
                "blockhash": "GHtXQBpokMJhbUyHQDiKvJvPchsb4xRuvfFwkdSEiMPQ",
                "lastValidBlockHeight": 123456
            }
        });
        let blockhash = parse_get_latest_blockhash_response(&raw).unwrap();
        assert_eq!(blockhash, "GHtXQBpokMJhbUyHQDiKvJvPchsb4xRuvfFwkdSEiMPQ");
    }

    #[test]
    fn parses_rent_exemption_response() {
        let raw = serde_json::json!(2282880);
        let rent = parse_get_minimum_balance_for_rent_exemption_response(&raw).unwrap();
        assert_eq!(rent, 2282880);
    }
}
