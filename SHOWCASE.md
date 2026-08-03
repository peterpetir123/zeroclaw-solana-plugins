# 🛡️ Showcase: Solana DeFi Guardian — Zero-Custody Token Security & Transaction Agent

## What It Does

The **Solana DeFi Guardian** is a self-hosted AI agent that enforces mandatory pre-flight security audits on any Solana token before constructing a transfer transaction. It implements a 4-stage Standard Operating Procedure (SOP) with a Human-in-the-Loop (HITL) approval gate between risk assessment and transaction construction.

**The pipeline:**
1. User requests a token transfer (via HTTP Gateway / REST API)
2. Agent calls `token-risk-check` plugin → scans mint on-chain for freeze authority, mint authority, permanent delegates, transfer hooks, Token-2022 extensions
3. **HITL gate** — SOP pauses at `waiting_approval`. Human operator reviews the Red/Amber/Green risk report and explicitly approves via CLI (`zeroclaw sop approve`)
4. Agent calls `spl-transfer-build` plugin → constructs an unsigned Versioned V0 Base64 transaction with auto-ATA creation
5. **Final HITL gate** — Agent presents the unsigned payload for operator signature authorization before external broadcast.

**No private keys are ever stored, requested, or processed by the agent.**

---

## Who It's For

- **DeFi operators** who want automated pre-flight security checks before any token movement
- **Treasury managers** using AI agents to draft multisig proposals with mandatory risk gates
- **Bot operators** preparing zero-custody transaction construction for payment channels
- **Anyone** who wants an AI agent that can touch Solana safely — auditing tokens and building transactions without ever holding keys

---

## ZeroClaw Features Used

| Feature | How It's Used |
|---|---|
| **WASM Plugins** (`wasm32-wasip2`) | Two custom plugins: `token-risk-check` (T0) and `spl-transfer-build` (T1), implementing the `tool-plugin` WIT world |
| **SOP Engine** | `solana-transfer-guard` SOP with 4 steps, `admission_policy = "hold"`, manual trigger |
| **HITL Approval Checkpoints** | `requires_confirmation: true` on step 2 and step 4 — agent cannot proceed without human operator approval |
| **Plugin Config Ingestion** | Plugins read `solana_rpc_url` from host-injected config via `config_read` permission |
| **Reliability Backoff** | `[reliability]` section with `provider_retries = 5`, `provider_backoff_ms = 15000` for LLM rate limit resilience |
| **Skill** | `skills/solana-guardian/SKILL.md` teaches the agent the audit→build workflow and prompt injection defense rules |
| **Persistent Memory** | SQLite backend for session state across runs |

---

## What We Had to Build

### `solana-lite` — Custom WASM-Compatible Solana Primitives

Standard Solana SDKs (`solana-sdk`, `solana-client`) depend on `tokio`, `sysinfo`, `net` — none of which compile for `wasm32-wasip2`. We built **`solana-lite`**, a zero-dependency Rust library:

- **Base58** encoder/decoder (no `bs58` dependency)
- **Ed25519 PDA & ATA derivation** using `curve25519-dalek` (off-curve check verified against on-chain USDC/WSOL ATAs)
- **Versioned Transaction V0 serializer** (`0x80` prefix, compact-u16 encoding)
- **Token-2022 TLV extension parser** (permanent delegate, transfer hook, transfer fee)
- **Minimalist JSON-RPC client** over `waki` (WASI HTTP)

**29 unit tests** cover all primitives with offline fixtures.

### Two WASM Plugins

| Plugin | Lines of Rust | Tests | WASM Size |
|---|---|---|---|
| `token-risk-check` | ~800 | 11 | ~173 KB |
| `spl-transfer-build` | ~600 | 9 | ~212 KB |

Both follow the reference plugin layout: pure Rust core + thin `#[cfg(target_family = "wasm")]` shim, host-run tests with mocked RPC, structured logging via the `logging` WIT import, manifest declaring only `http_client` + `config_read` permissions.

---

## Custody Tier & Threat Model

| Component | Tier | Secrets Held | Threat Surface |
|---|---|---|---|
| `token-risk-check` | **T0** (Read-Only) | RPC URL only | Read-only `getAccountInfo` calls. Invalid Base58 fails closed. |
| `spl-transfer-build` | **T1** (Unsigned Build) | RPC URL only | Constructs unsigned tx. Rejects non-numeric amounts ("all", "max", negatives). Never accepts private keys. |
| `solana-transfer-guard` SOP | **T0→T1 Pipeline** | None | HITL gate enforces human review between risk check and tx build. Agent cannot auto-execute. |

**Third-party trust:** Solana public RPC (`api.mainnet-beta.solana.com`). No external MCP servers. No key custody services. Fully self-hosted.

---

## Defense-in-Depth & Prompt Injection Security

We implement a 3-layer security model to prevent malicious drain attacks:

1. **Layer 1 (Rust Code Fail-Closed):** The `spl-transfer-build` plugin validates inputs strictly in Rust. Non-integer or natural language amounts (such as `"all"`, `"max"`, or negative numbers) cause a hard `ParseIntError` rejection before any RPC call or transaction assembly occurs (`test injection_via_amount_field_fails_closed ... ok`).
2. **Layer 2 (SOP HITL Checkpoints):** The SOP engine pauses at Step 2 and Step 4 with `requires_confirmation: true`. Even if an attacker tricks the LLM into calling a build tool, the transaction cannot be presented for signing without explicit operator CLI approval.
3. **Layer 3 (LLM Alignment):** The `solana-guardian` skill instructs the agent to recognize social engineering and refuse malicious instructions.

---

## Reproduction Guide & Setup Effort

### Estimated Setup Time
**~15-20 minutes** for a first-time operator:
- Rust toolchain + `wasm32-wasip2` target setup (~3 min)
- Building ZeroClaw host from source (~10-15 min depending on CPU)
- Plugin compilation & unit test suite execution (~2 min)

*Note on LLM Providers:* For multi-step SOP runs, we recommend **OpenRouter** (e.g. `gpt-4o-mini`) over strict free-tier Gemini endpoints to avoid aggressive 5 RPM rate limits. Groq works for small models like `llama-3.1-8b-instant` but has 12K TPM limits on free tier.

### Quick Start
```bash
# 1. Clone this repo
git clone https://github.com/peterpetir123/zeroclaw-solana-plugins.git
cd zeroclaw-solana-plugins

# 2. Run all 49 unit tests
(cd plugins/solana-lite && cargo test)
(cd plugins/token-risk-check && cargo test)
(cd plugins/spl-transfer-build && cargo test)

# 3. Live mainnet audit (no ZeroClaw needed)
(cd plugins/token-risk-check && cargo run --bin token-risk-check-cli EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)

# 4. Full host setup + WASM plugin verification
cp config.example.toml ~/.zeroclaw/config.toml
# Edit ~/.zeroclaw/config.toml — set your API key
./setup_and_run_zeroclaw.sh
```

### SOP Execution (With Running Daemon)

> **PREREQUISITE:** The ZeroClaw daemon must be running. Start it first:
> ```bash
> zeroclaw daemon &    # runs in background on port 42617
> sleep 3              # wait for gateway to start
> ```

```bash
# 1. Trigger SOP
RESULT=$(curl -s -X POST http://127.0.0.1:42617/api/sops/solana-transfer-guard/run \
  -H "Content-Type: application/json" \
  -d '{"payload": "{\"mint_address\": \"So11111111111111111111111111111111111111112\", \"from\": \"4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU\", \"to\": \"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8\", \"amount\": \"100000\"}"}')
echo "$RESULT"

# 2. Extract run_id (requires python3)
export RUN_ID=$(echo $RESULT | python3 -c "import sys,json; print(json.load(sys.stdin).get('run_id',''))")
echo "RUN_ID: $RUN_ID"

# 3. Check overlay state
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay

# 4. Approve HITL gate (repeat at each checkpoint — Step 2 and Step 4)
zeroclaw sop approve $RUN_ID
```

> **If you get `"SOP held (a run is already in flight)"`**: A previous run left a stale lock. See the Troubleshooting section in [README.md](./README.md#-troubleshooting).

---

## Known Limitations & Roadmap

- **SOP Concurrency:** The `admission_policy = "hold"` setting means only one run at a time. If a run is interrupted (daemon crash, Ctrl+C), you must clear the stale claim before triggering a new run (see README Troubleshooting).
- **Trigger Gateway:** Current interaction is via ZeroClaw's HTTP Gateway REST API (`/api/sops/...`). Direct chat channel integrations (Telegram/Discord bots) are planned on the roadmap.
- **Multisig Integration:** Future versions aim to interface directly with Squads v4 Proposer roles for automated multisig drafting.

---

## Links & Verifiable Proofs

- **GitHub:** [github.com/peterpetir123/zeroclaw-solana-plugins](https://github.com/peterpetir123/zeroclaw-solana-plugins)
- **Config:** [`config.example.toml`](./config.example.toml)
- **SOP Definition:** [`sops/solana-transfer-guard/`](./sops/solana-transfer-guard/)
- **Skill:** [`skills/solana-guardian/SKILL.md`](./skills/solana-guardian/SKILL.md)
- **Verified Proof Logs:**
  - Full E2E SOP Run (Completed 4/4 Steps): [`sop_run_proof_overlay_completed.json`](./sop_run_proof_overlay_completed.json)
  - SOP Trigger Proof: [`sop_run_proof_trigger_final.json`](./sop_run_proof_trigger_final.json)
  - Success Run Log: [`sop_run_proof_scenario1_success.log`](./sop_run_proof_scenario1_success.log)
  - Prompt Injection Defense: [`sop_run_proof_scenario3_prompt_injection.log`](./sop_run_proof_scenario3_prompt_injection.log)
