# spl-transfer-build

`spl-transfer-build` is a ZeroClaw WIT tool plugin (`wasm32-wasip2`) that constructs unsigned Versioned Solana Transactions (Base64) for SOL or SPL token transfers, ready for human/host approval.

## Features
- **SOL & SPL Transfers**: Builds transfer instructions for native SOL and SPL / Token-2022 tokens.
- **Automatic ATA Creation**: Checks whether the recipient's Associated Token Account exists on-chain and automatically prepends `CreateAssociatedTokenAccount` instruction if missing.
- **Memo Attachment**: Includes optional invoice or tracking memo as an opaque `Memo` instruction.
- **Human-Readable Summary**: Generates a clear, human-friendly summary suitable for Telegram/Discord approval gates.
- **Prompt Injection Defense**: `amount` MUST be a positive integer string in smallest units. Natural language keywords (e.g. "all", "max", "transfer everything") are rejected immediately. Recipient pubkeys are validated strictly with base58 decoding.

## Custody Tier: T1 (Unsigned Build)
This plugin operates under Tier T1 custody standards:
- **Zero Key Access**: Never holds or accesses private keys or signing capabilities.
- Produces only **unsigned base64 transaction payloads**.
- Signing and broadcast occur outside the Wasm sandbox via human approval gate or host hardware wallet.

## Building & Testing

```bash
# Host-side core unit testing
cargo test

# Build WASM component (target wasm32-wasip2)
cargo build --target wasm32-wasip2 --release
```

Binary size: **~212 KB**.
