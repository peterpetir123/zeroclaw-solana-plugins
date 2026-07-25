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

            let parsed_args = serde_json::from_str::<serde_json::Value>(&args).unwrap_or(serde_json::Value::Null);
            let config = parsed_args.get("__config");
            let rpc_url = match config
                .and_then(|c| c.get("solana_rpc_url"))
                .and_then(|v| v.as_str())
            {
                Some(url) => url.to_string(),
                None => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "missing solana_rpc_url config");
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
