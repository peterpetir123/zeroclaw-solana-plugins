//! Mock RPC implementation for host-side testing.
//!
//! Provides several modes:
//! - `from_fixture(path)`: loads JSON fixture file with account data
//! - `panics_if_called()`: panics on any RPC call (for verifying early rejection)
//! - `always_errors(msg)`: returns Err on any call (for error propagation tests)

use solana_lite::rpc::{AccountInfo, SolanaRpc};
use std::collections::HashMap;

/// Mock RPC for testing without network access.
pub struct MockRpc {
    mode: MockMode,
}

enum MockMode {
    Fixture(HashMap<String, serde_json::Value>),
    PanicsIfCalled,
    AlwaysErrors(String),
}

#[derive(serde::Deserialize)]
struct FixtureFile {
    accounts: HashMap<String, FixtureAccount>,
    #[serde(default = "default_blockhash")]
    blockhash: String,
}

fn default_blockhash() -> String {
    "GHtXQBpokMJhbUyHQDiKvJvPchsb4xRuvfFwkdSEiMPQ".to_string()
}

#[derive(serde::Deserialize)]
struct FixtureAccount {
    data_base64: String,
    owner: String,
    lamports: u64,
    #[serde(default)]
    executable: bool,
}

impl MockRpc {
    /// Load mock data from a JSON fixture file.
    pub fn from_fixture(path: &str) -> Self {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
        let raw: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("failed to parse fixture {path}: {e}"));
        let mut accounts = HashMap::new();
        accounts.insert("__fixture__".to_string(), raw);
        MockRpc {
            mode: MockMode::Fixture(accounts),
        }
    }

    /// Creates a mock that panics if any RPC method is called.
    /// Use to verify that validation rejects input before making RPC calls.
    pub fn panics_if_called() -> Self {
        MockRpc {
            mode: MockMode::PanicsIfCalled,
        }
    }

    /// Creates a mock that always returns an error with the given message.
    pub fn always_errors(msg: &str) -> Self {
        MockRpc {
            mode: MockMode::AlwaysErrors(msg.to_string()),
        }
    }
}

impl SolanaRpc for MockRpc {
    fn get_account_info(&self, _pubkey: &str) -> Result<Option<AccountInfo>, String> {
        match &self.mode {
            MockMode::Fixture(accounts) => {
                let fixture_val = accounts.get("__fixture__").unwrap();
                let fixture: FixtureFile = serde_json::from_value(fixture_val.clone())
                    .map_err(|e| format!("fixture parse error: {e}"))?;
                // Return first matching account or the first one
                if let Some(acct) = fixture.accounts.values().next() {
                    Ok(Some(AccountInfo {
                        data_base64: acct.data_base64.clone(),
                        owner: acct.owner.clone(),
                        lamports: acct.lamports,
                        executable: acct.executable,
                    }))
                } else {
                    Ok(None)
                }
            }
            MockMode::PanicsIfCalled => {
                panic!("MockRpc::get_account_info called when it should not have been (validation should have rejected input)")
            }
            MockMode::AlwaysErrors(msg) => Err(msg.clone()),
        }
    }

    fn get_latest_blockhash(&self) -> Result<String, String> {
        match &self.mode {
            MockMode::Fixture(accounts) => {
                let fixture_val = accounts.get("__fixture__").unwrap();
                let fixture: FixtureFile = serde_json::from_value(fixture_val.clone())
                    .map_err(|e| format!("fixture parse error: {e}"))?;
                Ok(fixture.blockhash)
            }
            MockMode::PanicsIfCalled => {
                panic!("MockRpc::get_latest_blockhash called when it should not have been")
            }
            MockMode::AlwaysErrors(msg) => Err(msg.clone()),
        }
    }

    fn get_minimum_balance_for_rent_exemption(&self, _size: u64) -> Result<u64, String> {
        match &self.mode {
            MockMode::Fixture(_) => Ok(2_039_280), // standard rent for Token account
            MockMode::PanicsIfCalled => {
                panic!("MockRpc::get_minimum_balance called when it should not have been")
            }
            MockMode::AlwaysErrors(msg) => Err(msg.clone()),
        }
    }
}
