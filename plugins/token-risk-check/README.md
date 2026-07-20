# token-risk-check

`token-risk-check` is a ZeroClaw WIT tool plugin (`wasm32-wasip2`) that assesses the security risk of a Solana SPL / Token-2022 mint address before an AI agent transacts with it.

## Features
- **Mint & Freeze Authority Check**: Detects active authorities that could mint supply or freeze accounts.
- **Token-2022 Extensions Evaluation**: Parses TLV extensions including `TransferFeeConfig`, `TransferHook`, `PermanentDelegate`, `DefaultAccountState`, and `NonTransferable`.
- **RAG Status Output**: Produces a clear `GREEN`, `AMBER`, or `RED` status with granular findings and an LLM-friendly summary capped at ~200 tokens.
- **Fail-Closed Security**: Rejects invalid base58 input, network errors, or malformed data before making assumptions.
- **Prompt Injection Defense**: All input fields are parsed as strictly-typed data (`Pubkey`), rejecting natural language or prompt manipulation attempts.

## Custody Tier: T0 (Read-Only)
This plugin operates under Tier T0 zero-custody standards:
- Requires only `http_client` and `config_read` capabilities in `manifest.toml`.
- Holds zero private keys or secret values.
- Performs only read-only RPC calls to assess account state.

## Building & Testing

```bash
# Host-side core unit testing (no Wasm dependency required)
cargo test

# Build WASM component (target wasm32-wasip2)
cargo build --target wasm32-wasip2 --release
```

Binary size: **~173 KB**.
