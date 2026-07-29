# Solana Transfer Guardian SOP

This SOP enforces a pre-transfer security audit on any Solana SPL/Token-2022 mint before constructing an unsigned transaction. It implements a zero-custody (T0→T1) pipeline: the agent never holds private keys, and a human must approve the final unsigned payload before signing externally.

## Steps

1. **Audit token mint risk** — Call token-risk-check plugin to scan target mint for freeze authorities, mint authorities, permanent delegates, and token extensions.
   - tools: token-risk-check
   - allow-tools: token-risk-check
   - next: 2

2. **Evaluate risk assessment checkpoint** — Inspect risk findings from step 1. Operator approval gate before transaction construction.
   - requires_confirmation: true
   - kind: checkpoint
   - next: 3

3. **Construct unsigned transaction** — Call spl-transfer-build plugin to construct unsigned Versioned V0 transaction (Base64) for token transfer.
   - tools: spl-transfer-build
   - allow-tools: spl-transfer-build
   - next: 4

4. **Human signature and broadcast checkpoint** — Present unsigned Base64 payload and human summary to operator for wallet signature authorization.
   - requires_confirmation: true
   - kind: checkpoint
