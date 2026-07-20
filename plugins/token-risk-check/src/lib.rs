//! A ZeroClaw WIT tool plugin: `token-risk-check`.
//!
//! Assesses the security risk of an SPL / Token-2022 token mint before
//! an agent transacts with it. Checks mint/freeze authority, Token-2022
//! extensions (transfer hooks, permanent delegate, transfer fees, etc.)
//! and returns a RAG (Red/Amber/Green) risk report.
//!
//! Custody tier: T0 (read-only). No secrets held beyond an RPC URL/key.
//!
//! The pure analysis core lives in [`core`] with no wasm dependency, so it
//! compiles and tests on the host with a plain `cargo test`; the wasm
//! component reuses the exact same logic through the shim below.
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

    use crate::core::analyzer::check_token;
    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};
    use zeroclaw::plugin::logging::{
        log_record, LogLevel, PluginAction, PluginEvent, PluginOutcome,
    };

    struct TokenRiskCheck;

    const PLUGIN_NAME: &str = "token-risk-check";
    const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
    const TOOL_NAME: &str = "token-risk-check";

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
            // Minimal wasi:http POST using the generated bindings
            // This is a placeholder — in actual deployment, use waki or
            // direct wasi:http/outgoing-handler calls
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

        fn get_latest_blockhash(&self) -> Result<String, String> {
            let result = self.rpc_call("getLatestBlockhash", serde_json::json!([]))?;
            result.get("value")
                .and_then(|v| v.get("blockhash"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| "failed to parse blockhash from response".to_string())
        }

        fn get_minimum_balance_for_rent_exemption(&self, size: u64) -> Result<u64, String> {
            let result = self.rpc_call("getMinimumBalanceForRentExemption", serde_json::json!([size]))?;
            result.as_u64()
                .ok_or_else(|| "failed to parse rent exemption amount".to_string())
        }
    }

    // ── WIT exports ──────────────────────────────────────────────────

    impl PluginInfo for TokenRiskCheck {
        fn plugin_name() -> String {
            PLUGIN_NAME.to_string()
        }

        fn plugin_version() -> String {
            PLUGIN_VERSION.to_string()
        }
    }

    impl Tool for TokenRiskCheck {
        fn name() -> String {
            TOOL_NAME.to_string()
        }

        fn description() -> String {
            "Assess the security risk of a Solana SPL/Token-2022 token mint before transacting. \
             Checks mint/freeze authority, Token-2022 extensions (transfer hooks, permanent \
             delegate, transfer fees), and returns a RAG (Red/Amber/Green) risk report. \
             Read-only, zero custody."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "mint_address": {
                        "type": "string",
                        "description": "Base58-encoded Solana token mint address to check"
                    }
                },
                "required": ["mint_address"],
                "additionalProperties": false
            })
            .to_string()
        }

        fn execute(args: String) -> Result<ToolResult, String> {
            emit(PluginAction::Start, PluginOutcome::Success, "execute called", None);

            let parsed: serde_json::Value = match serde_json::from_str(&args) {
                Ok(v) => v,
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "invalid arguments", None);
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("invalid arguments: {e}")),
                    });
                }
            };

            let mint_address = match parsed.get("mint_address").and_then(|v| v.as_str()) {
                Some(addr) => addr,
                None => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "missing mint_address", None);
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("field 'mint_address' is required and must be a string".to_string()),
                    });
                }
            };

            // Read RPC URL from host config
            let rpc_url = match crate::component::zeroclaw::plugin::logging::log_record {
                // Config is not directly available via logging import in tool-plugin world.
                // The host injects config via __config in args (same as redact-text pattern).
                _ => {
                    // Try to get RPC URL from __config in args
                    parsed.get("__config")
                        .and_then(|c| c.get("solana_rpc_url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("https://api.mainnet-beta.solana.com")
                        .to_string()
                }
            };

            let rpc = WakiRpc { _base_url: rpc_url };

            match check_token(&rpc, mint_address) {
                Ok(report) => {
                    let output = serde_json::to_string(&report)
                        .unwrap_or_else(|e| format!("{{\"error\": \"serialize failed: {e}\"}}"));
                    emit(PluginAction::Complete, PluginOutcome::Success, "check complete", None);
                    Ok(ToolResult {
                        success: true,
                        output,
                        error: None,
                    })
                }
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, &e, None);
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(e),
                    })
                }
            }
        }
    }

    fn emit(
        action: PluginAction,
        outcome: PluginOutcome,
        message: &str,
        attrs_json: Option<&str>,
    ) {
        log_record(
            LogLevel::Info,
            &PluginEvent {
                function_name: "token_risk_check::tool::execute".to_string(),
                action,
                outcome: Some(outcome),
                duration_ms: None,
                attrs: attrs_json.map(|s| s.to_string()),
                message: message.to_string(),
            },
        );
    }

    export!(TokenRiskCheck);
}
