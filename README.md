# ZeroClaw Solana Plugin Suite & SOP Orchestration

A suite of high-performance, zero-custody WebAssembly tool plugins (`wasm32-wasip2`) and automated Standard Operating Procedures (SOPs) for the **ZeroClaw AI Agent Runtime**, bringing Solana transaction capability, token security auditing, and Human-in-the-Loop (HITL) approval gates to autonomous AI agents.

---

## 📋 Comprehensive Write-Up & Technical Report

### 1. What It Does
The **ZeroClaw Solana Plugin Suite** equips autonomous AI agents with essential, secure tools and automated workflow guardrails:
- **`token-risk-check`**: Scans any SPL or Token-2022 mint on Solana to evaluate critical security risks (Freeze Authorities, Mint Authorities, Permanent Delegates, Transfer Hooks, and Fees). Returns a capped Red-Amber-Green (RAG) risk report.
- **`spl-transfer-build`**: Safely constructs unsigned, human-verifiable Solana Versioned Transactions (v0 Base64) for SOL and SPL token transfers, automatically detecting missing Associated Token Accounts (ATAs) and injecting `CreateIdempotent` instructions.
- **`solana-transfer-guard` (SOP)**: Multi-step governance workflow that mandates a pre-transfer token risk audit before assembling an unsigned transfer payload, enforceably paused by a Human-in-the-Loop (HITL) approval gate.

---

## 🏗️ Architecture & SOP Workflow Diagram

```
                             [ User / Trigger ]
                                     │
                                     ▼
                    ┌─────────────────────────────────┐
                    │  SOP: solana-transfer-guard     │
                    └─────────────────────────────────┘
                                     │
                                     ▼
                    ┌─────────────────────────────────┐
                    │ Step 1: Audit Token Mint Risk   │
                    │   Plugin: token-risk-check (T0) │
                    └─────────────────────────────────┘
                                     │
                                     ▼
                    ┌─────────────────────────────────┐
                    │ Step 2: HITL Approval Checkpoint│
                    │   Status: waiting_approval      │
                    └─────────────────────────────────┘
                                     │
                        [ Human Operator Approve ]
                                     │
                                     ▼
                    ┌─────────────────────────────────┐
                    │ Step 3: Build Unsigned Tx Payload│
                    │   Plugin: spl-transfer-build (T1)│
                    └─────────────────────────────────┘
                                     │
                                     ▼
                    ┌─────────────────────────────────┐
                    │ Step 4: Final Payload Delivery  │
                    │   Output: Unsigned Base64 V0 Tx │
                    └─────────────────────────────────┘
```

---

## 🔒 Custody Tier & Threat Model

| Component | Custody Tier | Threat Model & Security Controls |
|---|---|---|
| `token-risk-check` | **T0** (Read-Only) | **Zero Custody**. Read-only RPC calls. Input sanitization prevents prompt injection; invalid mint addresses fail closed immediately. |
| `spl-transfer-build` | **T1** (Unsigned Build) | **Zero Custody**. Constructs *unsigned* Base64 transactions. Does not store or accept private keys. Prevents relative amount exploits ("all", "max") by failing closed on non-numeric inputs. |
| `solana-transfer-guard` | **T0 → T1 Pipeline** | **Enforceable Approval Gate**. Enforces HITL approval between risk check and transaction construction, preventing automated execution of unverified token transfers. |

---

## ⚡ ZeroClaw Runtime & Reliability Backoff Policy

To ensure high-fidelity execution and resilience against API rate-limiting (e.g. Gemini 429 Rate Limits), ZeroClaw runtime's `config.toml` implements exponential backoff retry policies:

```toml
[reliability]
provider_retries = 5
provider_backoff_ms = 15000
```

When an LLM provider encounters rate limits (429), the `zeroclaw_providers::reliable` engine automatically schedules retries with backoff delays (15s, 10s, etc.), keeping SOP runs intact without failing the turn.

---

## 🚀 Quickstart & Verification Guide

> [!IMPORTANT]
> **All commands below must be run from the `zeroclaw-solana-plugins` directory**, not from the ZeroClaw host repo.
> ```bash
> cd /path/to/zeroclaw-solana-plugins   # ← this repo
> ```

### Step 1: Run Unit Test Suite (49 Tests)
Run all 49 failsafe security tests across the workspace:
```bash
(cd plugins/solana-lite && cargo test)         # 29 tests
(cd plugins/token-risk-check && cargo test)    # 11 tests
(cd plugins/spl-transfer-build && cargo test)  # 9 tests
```

---

### Step 2: Build WebAssembly (`wasm32-wasip2`) Component Binaries
Compile WASM components for production ZeroClaw runtime deployment:
```bash
(cd plugins/token-risk-check && cargo build --target wasm32-wasip2 --release)
(cd plugins/spl-transfer-build && cargo build --target wasm32-wasip2 --release)
```

---

### Step 3: Full Host Setup (Automated)
Use the provided setup script to build the host, install plugins, and verify discovery:
```bash
# Copy and edit config (set your LLM API key)
cp config.example.toml ~/.zeroclaw/config.toml
# Edit ~/.zeroclaw/config.toml — set your api_key

# Run automated setup
./setup_and_run_zeroclaw.sh
```

---

### Step 4: Execute SOP Governance Pipeline

> [!IMPORTANT]
> **The ZeroClaw daemon MUST be running** before triggering any SOP.
> Start it in a separate terminal or in the background:
> ```bash
> zeroclaw daemon &
> # Wait ~3 seconds for the HTTP gateway to start on port 42617
> ```

#### 4.1 Trigger SOP Run via REST API:
```bash
RESULT=$(curl -s -X POST http://127.0.0.1:42617/api/sops/solana-transfer-guard/run \
  -H "Content-Type: application/json" \
  -d '{"payload": "{\"mint_address\": \"So11111111111111111111111111111111111111112\", \"from\": \"4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU\", \"to\": \"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8\", \"amount\": \"100000\"}"}')
echo "$RESULT"
```
> *Expected: `{"run_id":"run-XXXXXXXXXX-0001"}`. If you get an empty response, the daemon is not running — see above.*

#### 4.2 Extract RUN_ID and Check Overlay State:
```bash
# Extract RUN_ID from the JSON response
export RUN_ID=$(echo $RESULT | python3 -c "import sys,json; print(json.load(sys.stdin).get('run_id',''))")
echo "RUN_ID: $RUN_ID"

# Query overlay status
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay
```

#### 4.3 Approve HITL Gates via ZeroClaw CLI:
```bash
# Approve gate (repeat when SOP parks at each HITL checkpoint)
zeroclaw sop approve $RUN_ID
```
> The SOP has HITL checkpoints at Step 2 and Step 4. You need to approve each one as the run progresses.

---

## 🔧 Troubleshooting

### "SOP held (a run is already in flight)"
This happens when the SOP `solana-transfer-guard` uses `admission_policy = "hold"` with `max_concurrent = 1`. If a previous run was interrupted (e.g., daemon killed mid-run), a stale concurrency lock remains in the database.

**Fix — clear the stale claim and restart:**
```bash
python3 -c "
import sqlite3
conn = sqlite3.connect('$HOME/.zeroclaw/data/sop/runs.db')
c = conn.cursor()
c.execute('DELETE FROM sop_claims')
c.execute('UPDATE sop_runs SET terminal = 1 WHERE terminal = 0')
conn.commit()
print('Stale SOP claims cleared.')
"

# Restart the daemon
pkill zeroclaw; sleep 1; zeroclaw daemon &
```

### Empty `$RUN_ID` / "required arguments not provided"
This means the `curl` POST returned an error instead of a `run_id`. Common causes:
1. **Daemon not running** — start it with `zeroclaw daemon &`
2. **SOP held** — clear stale claims (see above)
3. **Wrong directory** — ensure you're in `zeroclaw-solana-plugins/`, not `zeroclaw/`

### "cd: no such file or directory: plugins/solana-lite"
You are in the wrong directory. Plugin source code lives in **this repo** (`zeroclaw-solana-plugins/plugins/`), not in the ZeroClaw host repo.
```bash
cd /path/to/zeroclaw-solana-plugins   # ← correct
```

---

## 🛠️ Included Plugins & SOP Summary

| Item | Type | Description | Size / Config |
|---|---|---|---|
| [`token-risk-check`](./plugins/token-risk-check) | Plugin (WASM) | Assesses SPL / Token-2022 mint security returning RAG status. | **~173 KB** |
| [`spl-transfer-build`](./plugins/spl-transfer-build) | Plugin (WASM) | Constructs unsigned Versioned Tx v0 (Base64) for SOL & SPL transfers. | **~212 KB** |
| [`solana-transfer-guard`](./sops/solana-transfer-guard) | SOP Workflow | 4-step governance pipeline with HITL approval gate. | **SOP.toml / SOP.md** |

---

## ⚙️ Model Provider Recommendation

For multi-step SOP runs, use a provider with sufficient rate limits:

| Provider | Recommended For | Notes |
|---|---|---|
| **OpenRouter** (gpt-4o-mini, etc.) | Best overall for SOP runs | No aggressive TPM limits |
| **Groq** (llama-3.1-8b-instant) | Fast inference | Free tier has 12K TPM — use smaller models |
| **Gemini** (gemini-2.0-flash) | Quick tests | Free tier 5 RPM may fail multi-step SOPs |

All rate limits are handled by ZeroClaw's `[reliability]` provider retry backoff policies.

---

## 📄 License

MIT
