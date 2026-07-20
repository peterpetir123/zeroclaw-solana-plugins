//! Mock RPC implementation for host-side testing of spl-transfer-build.

use solana_lite::rpc::{AccountInfo, SolanaRpc};
use std::collections::HashMap;

/// Mock RPC for testing without network access.
pub struct MockRpc {
    mode: MockMode,
}

enum MockMode {
    Fixture(serde_json::Value),
    PanicsIfCalled,
    AlwaysErrors(String),
}

#[derive(serde::Deserialize)]
struct FixtureFile {
    /// Map of pubkey -> account info. If an ATA pubkey is absent, it means "not found".
    #[serde(default)]
    accounts: HashMap<String, FixtureAccount>,
    #[serde(default = "default_blockhash")]
    blockhash: String,
    /// If true, all get_account_info calls return None (ATA doesn't exist).
    #[serde(default)]
    ata_missing: bool,
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
    pub fn from_fixture(path: &str) -> Self {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
        let raw: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("failed to parse fixture {path}: {e}"));
        MockRpc {
            mode: MockMode::Fixture(raw),
        }
    }

    pub fn panics_if_called() -> Self {
        MockRpc {
            mode: MockMode::PanicsIfCalled,
        }
    }

    pub fn always_errors(msg: &str) -> Self {
        MockRpc {
            mode: MockMode::AlwaysErrors(msg.to_string()),
        }
    }
}

impl SolanaRpc for MockRpc {
    fn get_account_info(&self, pubkey: &str) -> Result<Option<AccountInfo>, String> {
        match &self.mode {
            MockMode::Fixture(raw) => {
                let fixture: FixtureFile = serde_json::from_value(raw.clone())
                    .map_err(|e| format!("fixture parse error: {e}"))?;
                if fixture.ata_missing {
                    return Ok(None);
                }
                // Try exact match first, then __any__ wildcard
                let acct = fixture.accounts.get(pubkey)
                    .or_else(|| fixture.accounts.get("__any__"));
                if let Some(acct) = acct {
                    Ok(Some(AccountInfo {
                        data_base64: acct.data_base64.clone(),
                        owner: acct.owner.clone(),
                        lamports: acct.lamports,
                        executable: acct.executable,
                    }))
                } else {
                    // Account not in fixture = doesn't exist on chain
                    Ok(None)
                }
            }
            MockMode::PanicsIfCalled => {
                panic!("MockRpc::get_account_info called when it should not have been")
            }
            MockMode::AlwaysErrors(msg) => Err(msg.clone()),
        }
    }

    fn get_latest_blockhash(&self) -> Result<String, String> {
        match &self.mode {
            MockMode::Fixture(raw) => {
                let fixture: FixtureFile = serde_json::from_value(raw.clone())
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
            MockMode::Fixture(_) => Ok(2_039_280),
            MockMode::PanicsIfCalled => {
                panic!("MockRpc::get_minimum_balance called when it should not have been")
            }
            MockMode::AlwaysErrors(msg) => Err(msg.clone()),
        }
    }
}
