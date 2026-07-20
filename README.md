# ZeroClaw Solana Plugin Suite

A suite of high-performance, secure WebAssembly tool plugins (`wasm32-wasip2`) for the ZeroClaw AI agent runtime, bringing Solana transaction capability and security auditing to autonomous agents.

## Included Plugins

| Plugin | Custody Tier | Description | Binary Size |
|---|---|---|---|
| [`token-risk-check`](./plugins/token-risk-check) | **T0** (Read-Only) | Assesses SPL / Token-2022 mint security (Authorities, Token-2022 Extensions, Hooks, Fees) returning RAG status. | **~173 KB** |
| [`spl-transfer-build`](./plugins/spl-transfer-build) | **T1** (Unsigned Build) | Constructs unsigned Solana transactions (Base64) for SOL & SPL transfers with auto-ATA creation and human summary. | **~212 KB** |

## Architecture & Security Principles

1. **Pure Core, Thin Shim**: Business logic lives in standard Rust crates (`rlib`) tested natively via `cargo test` with mock RPC fixtures. The Wasm component is a thin glue layer exposing the `wit/v0` interface.
2. **Lightweight Solana Stack (`solana-lite`)**: Zero dependency on heavy `solana-sdk`/`solana-client` crates (which fail on `wasm32-wasip2`). Built with custom minimal wire serialization (`bs58`, `borsh`, manual compact-u16).
3. **Fail-Closed & Prompt Injection Defense**: Strictly-typed input validation. Relative amount keywords ("all", "max") and invalid pubkeys fail closed prior to RPC invocation.
4. **Minimal Permissions**: `manifest.toml` declares only `http_client` and `config_read` capabilities.

## License

MIT
