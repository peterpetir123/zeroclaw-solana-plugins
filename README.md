# ZeroClaw Solana Plugin Suite

A suite of high-performance, zero-custody WebAssembly tool plugins (`wasm32-wasip2`) for the ZeroClaw AI agent runtime, bringing Solana transaction capability and security auditing to autonomous agents.

---

## 📋 Comprehensive Write-Up & Technical Report

### 1. What It Does
The **ZeroClaw Solana Plugin Suite** equips autonomous AI agents with two essential, secure tools:
- **`token-risk-check`**: Scans any SPL or Token-2022 mint on Solana to evaluate critical security risks (Freeze Authorities, Mint Authorities, Permanent Delegates, Transfer Hooks, and Fees). Returns a capped Red-Amber-Green (RAG) risk report.
- **`spl-transfer-build`**: Safely constructs unsigned, human-verifiable Solana Versioned Transactions (v0 Base64) for SOL and SPL token transfers, automatically detecting missing Associated Token Accounts (ATAs) and injecting `CreateIdempotent` instructions.

### 2. Who It's For
Designed for **autonomous agent operators, DeFi trading bots, and AI assistant channels (Telegram, Discord, Terminal)** operating on Solana. It enables AI agents to query token risks and assemble transactions without ever holding or touching private key custody.

### 3. ZeroClaw Features & Skill Integration
- **WIT v0 Component Model (`wasm32-wasip2`)**: Exposes native `wit/v0` tool execution interfaces.
- **WASI HTTP Client (`waki`)**: Performs outbound JSON-RPC queries directly through WASI network interfaces.
- **Skill Registration (`zeroclaw skills install`)**: Fully compatible with ZeroClaw CLI skill/plugin loader.
- **Runtime Capability & Config Ingestion (`__config`)**: Consumes host-injected `SOLANA_RPC_URL` under strict permissions (`http_client`, `config_read`).

### 4. What We Had to Build (`solana-lite`)
Standard Solana SDKs (`solana-sdk`, `solana-client`) fail to compile on `wasm32-wasip2` due to heavy OS-level dependencies (`tokio`, `sysinfo`, `net`). We built **`solana-lite`**, a custom lightweight Rust library:
- Zero-dependency Base58 parser & encoder.
- Off-curve Ed25519 PDA & ATA address derivation using `curve25519-dalek`.
- Minimalist Versioned Transaction v0 byte serializer (`0x80` version prefix, compact-u16 format).
- Token-2022 Type-Length-Value (TLV) extension parser.

### 5. Custody Tier & Threat Model
| Plugin | Custody Tier | Threat Model & Security Controls |
|---|---|---|
| `token-risk-check` | **T0** (Read-Only) | **Zero Custody**. Read-only RPC calls. Input sanitization prevents prompt injection; invalid mint addresses fail closed immediately. |
| `spl-transfer-build` | **T1** (Unsigned Build) | **Zero Custody**. Constructs *unsigned* Base64 transactions. Does not store or accept private keys. Prevents relative amount exploits ("all", "max") by failing closed on non-numeric inputs. |

---

## 🚀 Execution & Verification Guide for Operators & Judges

### Step 1: Set Solana RPC URL (Optional)
By default, execution targets Solana Mainnet (`https://api.mainnet-beta.solana.com`). You can set a custom RPC via environment variable:
```bash
export SOLANA_RPC_URL="https://api.mainnet-beta.solana.com"
```

---

### Step 2: Live Mainnet Functional Execution (Direct Host Binaries)

#### 1. Live Token Risk Audit (`token-risk-check`):
Audit any token mint directly on Solana Mainnet (e.g. USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`):
```bash
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins/plugins/token-risk-check
cargo run --bin token-risk-check-cli EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```

#### 2. Live Unsigned Transaction Construction (`spl-transfer-build`):
Construct an unsigned Versioned V0 transaction directly using live Mainnet blockhashes and rent exemptions:
```bash
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins/plugins/spl-transfer-build
cargo run --bin spl-transfer-build-cli 8UQUJWj4XnYFaAZjP79SGiwmrcT3fuy3pD7ig5B5bjW2 EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v 1000000
```

---

### Step 3: Run Unit Test Suite (49 Tests)
Run all 49 failsafe security tests across the workspace:
```bash
# 1. solana-lite (29 tests)
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins/plugins/solana-lite && cargo test

# 2. token-risk-check (11 tests)
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins/plugins/token-risk-check && cargo test

# 3. spl-transfer-build (9 tests)
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins/plugins/spl-transfer-build && cargo test
```

---

### Step 4: Build WebAssembly (`wasm32-wasip2`) Release Binaries
Compile the WebAssembly components for production ZeroClaw runtime deployment:
```bash
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins/plugins/token-risk-check && cargo build --target wasm32-wasip2 --release
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins/plugins/spl-transfer-build && cargo build --target wasm32-wasip2 --release
```

---

### Step 5: Install Plugins into ZeroClaw CLI Runtime
ZeroClaw CLI (v0.8.3+) uses `zeroclaw skills` subcommands to manage plugins and skills:

```bash
# Install both skills into ZeroClaw
zeroclaw skills install /home/hengkerprotzy/coding/zeroclaw-solana-plugins/plugins/token-risk-check
zeroclaw skills install /home/hengkerprotzy/coding/zeroclaw-solana-plugins/plugins/spl-transfer-build

# Verify registered skills
zeroclaw skills list
```

---

## 🛠️ Included Plugins Summary

| Plugin | Custody Tier | Description | Binary Size |
|---|---|---|---|
| [`token-risk-check`](./plugins/token-risk-check) | **T0** (Read-Only) | Assesses SPL / Token-2022 mint security returning RAG status. | **~173 KB** |
| [`spl-transfer-build`](./plugins/spl-transfer-build) | **T1** (Unsigned Build) | Constructs unsigned Versioned Tx v0 (Base64) for SOL & SPL transfers. | **~212 KB** |

---

## 🧪 Test Suite & Verification

- **49 / 49 Unit Tests Passed (100% Pass Rate)**
- Built & validated for target **`wasm32-wasip2`** without warnings.
- Installed & audited under ZeroClaw CLI runtime.

## 📄 License

MIT
