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
        base_url: String,
        api_key: Option<String>,
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

        fn http_post(&self, body: &[u8]) -> Result<Vec<u8>, String> {
            let client = waki::Client::new();
            let mut req = client.post(&self.base_url)
                .header("Content-Type", "application/json");

            if let Some(key) = &self.api_key {
                req = req.header("Authorization", &format!("Bearer {key}"));
            }

            let response = req
                .body(body.to_vec())
                .send()
                .map_err(|e| format!("HTTP post failed to send: {e}"))?;

            let status = response.status_code();
            if status >= 400 {
                return Err(format!("RPC HTTP status error: {status}"));
            }

            response.body()
                .map_err(|e| format!("failed to read HTTP body: {e}"))
        }
    }

    impl SolanaRpc for WakiRpc {
        fn get_account_info(&self, pubkey: &str) -> Result<Option<AccountInfo>, String> {
            let params = serde_json::json!([pubkey, {"encoding": "base64"}]);
            let result = self.rpc_call("getAccountInfo", params)?;
            solana_lite::rpc::parse_get_account_info_response(&result)
        }

        fn get_latest_blockhash(&self) -> Result<String, String> {
            let result = self.rpc_call("getLatestBlockhash", serde_json::json!([]))?;
            solana_lite::rpc::parse_get_latest_blockhash_response(&result)
        }

        fn get_minimum_balance_for_rent_exemption(&self, size: u64) -> Result<u64, String> {
            let result = self.rpc_call("getMinimumBalanceForRentExemption", serde_json::json!([size]))?;
            solana_lite::rpc::parse_get_minimum_balance_for_rent_exemption_response(&result)
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

            // Read RPC config from __config (host injects this)
            let config = parsed.get("__config");
            let rpc_url = match config
                .and_then(|c| c.get("solana_rpc_url"))
                .and_then(|v| v.as_str())
            {
                Some(url) => url.to_string(),
                None => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "missing solana_rpc_url config", None);
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some("Configuration 'solana_rpc_url' is required but not provided in __config.".to_string()),
                    });
                }
            };

            let api_key = config
                .and_then(|c| c.get("solana_rpc_api_key").or_else(|| c.get("api_key")))
                .and_then(|v| v.as_str())
                .map(String::from);

            let rpc = WakiRpc { base_url: rpc_url, api_key };

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
