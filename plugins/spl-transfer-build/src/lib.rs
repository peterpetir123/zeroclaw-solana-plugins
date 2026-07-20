//! A ZeroClaw WIT tool plugin: `spl-transfer-build`.
//!
//! Builds unsigned versioned Solana transactions for SOL or SPL token
//! transfers, including automatic ATA creation and memo attachment.
//!
//! Custody tier: T1 (unsigned build only). This plugin NEVER holds or sees
//! a private key. The output is always an unsigned transaction (base64);
//! signing is done by a human or host outside the wasm sandbox.
//!
//! The pure builder core lives in [`core`] with no wasm dependency, so it
//! compiles and tests on the host with a plain `cargo test`.
//!
//! Build:  rustup target add wasm32-wasip2
//!         cargo build --target wasm32-wasip2 --release

pub mod core;

#[cfg(target_family = "wasm")]
mod component {
    wit_bindgen::generate!({
        path: "../../wit/v0",
        world: "tool-plugin",
        features: ["plugins-wit-v0"],
    });

    use crate::core::builder::build_unsigned_tx;
    use crate::core::model::TransferRequest;
    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};
    use zeroclaw::plugin::logging::{
        log_record, LogLevel, PluginAction, PluginEvent, PluginOutcome,
    };

    struct SplTransferBuild;

    const PLUGIN_NAME: &str = "spl-transfer-build";
    const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
    const TOOL_NAME: &str = "spl-transfer-build";

    // ── WakiRpc: SolanaRpc implementation over wasi:http ──────────────

    use solana_lite::rpc::{AccountInfo, SolanaRpc};

    struct WakiRpc {
        _base_url: String,
    }

    impl WakiRpc {
        fn rpc_call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            });
            let body_bytes = serde_json::to_vec(&body)
                .map_err(|e| format!("json serialize error: {e}"))?;

            let resp_bytes = self.http_post(&body_bytes)?;

            let resp: serde_json::Value = serde_json::from_slice(&resp_bytes)
                .map_err(|e| format!("json parse error: {e}"))?;

            if let Some(error) = resp.get("error") {
                return Err(format!("RPC error: {}", error));
            }

            resp.get("result")
                .cloned()
                .ok_or_else(|| "RPC response missing 'result' field".to_string())
        }

        fn http_post(&self, _body: &[u8]) -> Result<Vec<u8>, String> {
            Err("HTTP not available in this build".to_string())
        }
    }

    impl SolanaRpc for WakiRpc {
        fn get_account_info(&self, pubkey: &str) -> Result<Option<AccountInfo>, String> {
            let params = serde_json::json!([pubkey, {"encoding": "base64"}]);
            let result = self.rpc_call("getAccountInfo", params)?;

            let value = result.get("value");
            match value {
                None | Some(serde_json::Value::Null) => Ok(None),
                Some(val) => {
                    let data_arr = val.get("data")
                        .and_then(|d| d.as_array())
                        .ok_or("missing data array")?;
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

                    Ok(Some(AccountInfo { data_base64, owner, lamports, executable }))
                }
            }
        }

        fn get_latest_blockhash(&self) -> Result<String, String> {
            let result = self.rpc_call("getLatestBlockhash", serde_json::json!([]))?;
            result.get("value")
                .and_then(|v| v.get("blockhash"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| "failed to parse blockhash".to_string())
        }

        fn get_minimum_balance_for_rent_exemption(&self, size: u64) -> Result<u64, String> {
            let result = self.rpc_call("getMinimumBalanceForRentExemption", serde_json::json!([size]))?;
            result.as_u64()
                .ok_or_else(|| "failed to parse rent amount".to_string())
        }
    }

    // ── WIT exports ──────────────────────────────────────────────────

    impl PluginInfo for SplTransferBuild {
        fn plugin_name() -> String {
            PLUGIN_NAME.to_string()
        }

        fn plugin_version() -> String {
            PLUGIN_VERSION.to_string()
        }
    }

    impl Tool for SplTransferBuild {
        fn name() -> String {
            TOOL_NAME.to_string()
        }

        fn description() -> String {
            "Build an unsigned Solana transaction for transferring SOL or SPL tokens. \
             Returns base64-encoded unsigned transaction bytes for human/host signing. \
             Handles ATA creation and memo attachment. Never touches private keys."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "from": {
                        "type": "string",
                        "description": "Sender's base58 Solana pubkey (also fee payer)"
                    },
                    "to": {
                        "type": "string",
                        "description": "Recipient's base58 Solana pubkey"
                    },
                    "amount": {
                        "type": "string",
                        "description": "Amount in smallest units (lamports for SOL, raw amount for SPL). Must be a positive integer."
                    },
                    "mint": {
                        "type": "string",
                        "description": "SPL token mint address (base58). Omit for native SOL transfer."
                    },
                    "memo": {
                        "type": "string",
                        "description": "Optional memo text for invoice reconciliation"
                    }
                },
                "required": ["from", "to", "amount"],
                "additionalProperties": false
            })
            .to_string()
        }

        fn execute(args: String) -> Result<ToolResult, String> {
            emit(PluginAction::Start, PluginOutcome::Success, "execute called");

            let req: TransferRequest = match serde_json::from_str(&args) {
                Ok(r) => r,
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "invalid arguments");
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("invalid arguments: {e}")),
                    });
                }
            };

            let rpc_url = serde_json::from_str::<serde_json::Value>(&args)
                .ok()
                .and_then(|v| v.get("__config")?.get("solana_rpc_url")?.as_str().map(String::from))
                .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_string());

            let rpc = WakiRpc { _base_url: rpc_url };

            match build_unsigned_tx(&rpc, &req) {
                Ok(result) => {
                    let output = serde_json::to_string(&result)
                        .unwrap_or_else(|e| format!("{{\"error\": \"serialize failed: {e}\"}}"));
                    emit(PluginAction::Complete, PluginOutcome::Success, "build complete");
                    Ok(ToolResult {
                        success: true,
                        output,
                        error: None,
                    })
                }
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, &e);
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(e),
                    })
                }
            }
        }
    }

    fn emit(action: PluginAction, outcome: PluginOutcome, message: &str) {
        log_record(
            LogLevel::Info,
            &PluginEvent {
                function_name: "spl_transfer_build::tool::execute".to_string(),
                action,
                outcome: Some(outcome),
                duration_ms: None,
                attrs: None,
                message: message.to_string(),
            },
        );
    }

    export!(SplTransferBuild);
}
