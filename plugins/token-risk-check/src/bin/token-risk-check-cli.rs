#[cfg(not(target_family = "wasm"))]
use solana_lite::rpc::{parse_get_account_info_response, AccountInfo, SolanaRpc};
#[cfg(not(target_family = "wasm"))]
use std::env;
#[cfg(not(target_family = "wasm"))]
use token_risk_check::core::analyzer::check_token;

#[cfg(not(target_family = "wasm"))]
struct HostRpc {
    rpc_url: String,
}

#[cfg(not(target_family = "wasm"))]
impl SolanaRpc for HostRpc {
    fn get_account_info(&self, pubkey: &str) -> Result<Option<AccountInfo>, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [pubkey, {"encoding": "base64"}]
        });

        let resp: serde_json::Value = ureq::post(&self.rpc_url)
            .send_json(body)
            .map_err(|e| format!("HTTP request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("Failed to parse JSON response: {e}"))?;

        if let Some(err) = resp.get("error") {
            return Err(format!("RPC error: {err}"));
        }

        let result = resp.get("result").ok_or("Missing result in RPC response")?;
        parse_get_account_info_response(result)
    }

    fn get_latest_blockhash(&self) -> Result<String, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": []
        });

        let resp: serde_json::Value = ureq::post(&self.rpc_url)
            .send_json(body)
            .map_err(|e| format!("HTTP request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("Failed to parse JSON response: {e}"))?;

        let result = resp.get("result").ok_or("Missing result in RPC response")?;
        solana_lite::rpc::parse_get_latest_blockhash_response(result)
    }

    fn get_minimum_balance_for_rent_exemption(&self, size: u64) -> Result<u64, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getMinimumBalanceForRentExemption",
            "params": [size]
        });

        let resp: serde_json::Value = ureq::post(&self.rpc_url)
            .send_json(body)
            .map_err(|e| format!("HTTP request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("Failed to parse JSON response: {e}"))?;

        let result = resp.get("result").ok_or("Missing result in RPC response")?;
        solana_lite::rpc::parse_get_minimum_balance_for_rent_exemption_response(result)
    }
}

fn main() {
    #[cfg(not(target_family = "wasm"))]
    {
        let args: Vec<String> = env::args().collect();
        let mint_address = args.get(1).map(|s| s.as_str()).unwrap_or("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        let rpc_url = env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

        println!("🔍 Auditing token mint on Solana Mainnet: {}", mint_address);
        println!("🌐 RPC URL: {}\n", rpc_url);

        let rpc = HostRpc { rpc_url };

        match check_token(&rpc, mint_address) {
            Ok(report) => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            }
            Err(e) => {
                eprintln!("❌ Audit failed: {}", e);
                std::process::exit(1);
            }
        }
    }
    #[cfg(target_family = "wasm")]
    {
        println!("CLI binary is for host execution only.");
    }
}
