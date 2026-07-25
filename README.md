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

### 3. ZeroClaw Features Used
- **WIT v0 Component Model (`wasm32-wasip2`)**: Exposes native `wit/v0` tool execution interfaces.
- **WASI HTTP Client (`waki`)**: Performs outbound JSON-RPC queries directly through WASI network interfaces.
- **Runtime Capability & Config Ingestion (`__config`)**: Consumes host-injected `solana_rpc_url` and `solana_rpc_api_key` under strict permissions (`http_client`, `config_read`).

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

### 6. Reproduction Guide for Operators & Judges

Operators and judges can verify each plugin individually using standard Rust / Cargo tools:

#### Step 1: Clone Repository
```bash
git clone https://github.com/peterpetir123/zeroclaw-solana-plugins.git
cd zeroclaw-solana-plugins
```

#### Step 2: Run Unit Tests for Each Plugin Individually
* **1. `solana-lite` (Shared Cryptography & V0 Serializer - 29 tests):**
  ```bash
  cd plugins/solana-lite && cargo test && cd ../..
  ```
* **2. `token-risk-check` (T0 Security Auditor - 11 tests):**
  ```bash
  cd plugins/token-risk-check && cargo test && cd ../..
  ```
* **3. `spl-transfer-build` (T1 Unsigned Transaction Builder - 9 tests):**
  ```bash
  cd plugins/spl-transfer-build && cargo test && cd ../..
  ```

#### Step 3: Build WebAssembly (`wasm32-wasip2`) Binaries
* **Build `token-risk-check.wasm`:**
  ```bash
  cd plugins/token-risk-check && cargo build --target wasm32-wasip2 --release && cd ../..
  ```
* **Build `spl-transfer-build.wasm`:**
  ```bash
  cd plugins/spl-transfer-build && cargo build --target wasm32-wasip2 --release && cd ../..
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

## 📄 License

MIT
