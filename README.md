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

> [!NOTE]
> Ensure you are inside the root directory of `zeroclaw-solana-plugins` repository before executing the commands below.

### Step 1: Run Unit Test Suite (49 Tests)
Run all 49 failsafe security tests across the workspace:
```bash
# 1. solana-lite (29 tests)
(cd plugins/solana-lite && cargo test)

# 2. token-risk-check (11 tests)
(cd plugins/token-risk-check && cargo test)

# 3. spl-transfer-build (9 tests)
(cd plugins/spl-transfer-build && cargo test)
```

---

### Step 2: Build WebAssembly (`wasm32-wasip2`) Component Binaries
Compile WASM components for production ZeroClaw runtime deployment:
```bash
(cd plugins/token-risk-check && cargo build --target wasm32-wasip2 --release)
(cd plugins/spl-transfer-build && cargo build --target wasm32-wasip2 --release)
```

---

### Step 3: Install Plugins & SOP into ZeroClaw CLI Runtime
ZeroClaw CLI uses `zeroclaw skills` subcommands to manage WASM plugins:

```bash
# 1. Install skills into ZeroClaw
zeroclaw skills install ./plugins/token-risk-check
zeroclaw skills install ./plugins/spl-transfer-build

# 2. Verify registered skills
zeroclaw skills list
```

---

### Step 4: Execute SOP Governance Pipeline

#### 1. Trigger SOP Run via REST API:
```bash
curl -s -X POST http://127.0.0.1:42617/api/sops/solana-transfer-guard/run \
  -H "Content-Type: application/json" \
  -d '{"payload": "{\"mint_address\": \"So11111111111111111111111111111111111111112\", \"from\": \"4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU\", \"to\": \"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8\", \"amount\": \"100000\"}"}'
```
> *This command returns a JSON response containing the unique `run_id` (e.g. `{"run_id":"run-1785421255609729742-0001"}`).*

#### 2. Check Run Overlay State:
You can check the overlay state using either of the following two methods:

##### **Method 1: Export RUN_ID to Terminal Variable**
```bash
# 1. Export the run_id returned from step 1
export RUN_ID="run-1785421255609729742-0001"

# 2. Query overlay status using the variable
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay
```

##### **Method 2: Query Directly Using Run ID String**
```bash
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/run-1785421255609729742-0001/overlay
```

#### 3. Approve HITL Gate via ZeroClaw CLI:
```bash
zeroclaw sop approve $RUN_ID
# Or directly: zeroclaw sop approve run-1785421255609729742-0001
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

- **Anthropic Claude (Sonnet/Opus)** — Recommended for production due to superior instruction-following and tool-use reliability for financial transaction approval gates.
- **Google Gemini / OpenAI / OpenRouter** — Fully supported. Rate limits are handled by ZeroClaw's `[reliability]` provider retry backoff policies.

---

## 📄 License

MIT
