# 📖 Tutorial: Solana DeFi Guardian Execution Guide

A step-by-step tutorial to run Solana token security audits, configure the ZeroClaw AI agent runtime, and execute the Standard Operating Procedure (SOP) governance workflow with Human-in-the-Loop (HITL) protection.

---

## 📋 System Prerequisites

1. **Rust Toolchain**: `rustc` and `cargo` with the WebAssembly `wasm32-wasip2` compilation target:
   ```bash
   rustup target add wasm32-wasip2
   ```
2. **Python 3**: For basic JSON parsing and database maintenance.
3. **Repository Workspace**:
   - Plugin Repository (this repo): `/home/hengkerprotzy/coding/zeroclaw-solana-plugins`
   - ZeroClaw Host Repository: `/home/hengkerprotzy/coding/zeroclaw`

---

## 🚀 Step 1: Run Core Failsafe Unit Tests & Build WASM Components

Before launching the ZeroClaw daemon, verify that all 49 security unit tests pass:

```bash
# 1. Enter the plugin repository directory
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins

# 2. Run unit tests across all three crates
(cd plugins/solana-lite && cargo test)
(cd plugins/token-risk-check && cargo test)
(cd plugins/spl-transfer-build && cargo test)

# 3. Build WebAssembly binaries (wasm32-wasip2 target)
(cd plugins/token-risk-check && cargo build --target wasm32-wasip2 --release)
(cd plugins/spl-transfer-build && cargo build --target wasm32-wasip2 --release)
```

---

## 🛡️ Step 2: Live Mainnet Audit via CLI (Read-Only Risk Check)

You can audit any Solana token mint directly on mainnet without running the full ZeroClaw daemon:

```bash
# Example: Audit USDC mint address on Solana Mainnet
(cd plugins/token-risk-check && cargo run --bin token-risk-check-cli EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
```

---

## ⚙️ Step 3: Database Cleanup & Launching ZeroClaw Daemon

```bash
# 1. Clear any stale concurrency locks in the SQLite database (from interrupted previous runs)
python3 -c "
import sqlite3, os
db_path = os.path.expanduser('~/.zeroclaw/data/sop/runs.db')
if os.path.exists(db_path):
    conn = sqlite3.connect(db_path)
    c = conn.cursor()
    c.execute('DELETE FROM sop_claims')
    c.execute('UPDATE sop_runs SET terminal = 1 WHERE terminal = 0')
    conn.commit()
    print('✅ Database synced and stale claims cleared.')
"

# 2. Ensure previous daemon instances are stopped
pkill -9 zeroclaw 2>/dev/null || true
sleep 1

# 3. Launch ZeroClaw Daemon in background
(cd /home/hengkerprotzy/coding/zeroclaw && ./target/release/zeroclaw daemon -v > /home/hengkerprotzy/coding/zeroclaw-solana-plugins/daemon_live.log 2>&1 &)

# 4. Verify daemon health status
sleep 3
curl -s http://127.0.0.1:42617/health | python3 -c "import sys,json; d=json.load(sys.stdin); print('Daemon Status:', d['status'], '| PID:', d['runtime']['pid'])"
```

---

## 🔄 Step 4: Execute End-to-End SOP Governance Pipeline (`solana-transfer-guard`)

The SOP enforces a 4-stage pipeline containing 2 mandatory Human-in-the-Loop (HITL) approval checkpoints:

### Step 4.1: Trigger New SOP Run via HTTP Gateway
```bash
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins

RESULT=$(curl -s -X POST http://127.0.0.1:42617/api/sops/solana-transfer-guard/run \
  -H "Content-Type: application/json" \
  -d '{"payload": "{\"mint_address\": \"So11111111111111111111111111111111111111112\", \"from\": \"4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU\", \"to\": \"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8\", \"amount\": \"100000\"}"}')

echo "Trigger Response: $RESULT"

# Export RUN_ID to environment variable
export RUN_ID=$(echo $RESULT | python3 -c "import sys,json; print(json.load(sys.stdin).get('run_id',''))")
echo "Active RUN_ID: $RUN_ID"
```

### Step 4.2: Check Overlay State & Approve First HITL Gate (Step 2)
```bash
# 1. Query overlay status (Step 1 completed, Step 2 waiting_approval)
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool

# 2. Grant operator approval to proceed to Step 3 (Build Tx)
/home/hengkerprotzy/coding/zeroclaw/target/release/zeroclaw sop approve $RUN_ID
```

### Step 4.3: Polling & Approve Second HITL Gate (Step 4)
```bash
# Wait ~8-10 seconds for Step 3 to assemble the unsigned transaction payload
sleep 8

# Query overlay status (Step 4 waiting_approval)
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool

# Grant final operator approval to finalize the SOP run
/home/hengkerprotzy/coding/zeroclaw/target/release/zeroclaw sop approve $RUN_ID
```

### Step 4.4: Verify Final SOP Completion (`completed`)
```bash
sleep 3

# Check final overlay status: all 4 nodes must show 'completed' state
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool
```

---

## 🔍 Step 5: Test Prompt Injection Security Defenses

Verify the plugin's fail-closed behavior against malicious natural language input (e.g. transfer `"all"` or `"max"`):

```bash
# Run WASM-level Rust security unit tests
(cd plugins/spl-transfer-build && cargo test injection_via_amount_field_fails_closed -- --nocapture)
(cd plugins/token-risk-check && cargo test prompt_injection_in_mint_address_fails_closed -- --nocapture)
```

---

## 🛠️ Quick Troubleshooting Guide

| Issue / Error | Root Cause | Solution |
|---|---|---|
| `RESULT: {"error":"SOP held (a run is already in flight)"}` | Stale concurrency claim in SQLite DB | Run the database cleanup script in Step 3, then kill and restart the daemon. |
| `error: required argument <RUN_ID> not provided` | Empty `$RUN_ID` because daemon was unreachable during `curl` call | Ensure daemon is running (`curl http://127.0.0.1:42617/health`), then re-run Step 4.1. |
| `cd: no such file or directory: plugins/...` | Shell is in `zeroclaw` directory instead of `zeroclaw-solana-plugins` | Run `cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins` first. |
