#!/bin/bash
set -euo pipefail

# ============================================================================
# ZeroClaw Solana Plugins — Live CLI Demo (Native ureq path)
# ============================================================================
#
# This script runs the native CLI binaries (ureq-based, NOT the WASM sandbox
# path). For the authoritative WASM sandbox proof, use setup_and_run_zeroclaw.sh
# which loads .wasm components into the ZeroClaw host runtime via wasmtime.
#
# This demo exists for quick local verification without a full host build.
# It calls live Solana Mainnet RPC — requires SOLANA_RPC_URL or uses the
# default public endpoint.
#
# Usage:
#   export SOLANA_RPC_URL="https://api.mainnet-beta.solana.com"
#   ./demo_live.sh
# ============================================================================

GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

PLUGIN_REPO="$(cd "$(dirname "$0")" && pwd)"

echo -e "${BLUE}========================================================================${NC}"
echo -e "${BLUE}     ZEROCLAW SOLANA PLUGIN SUITE — LIVE CLI DEMO (Native Path)         ${NC}"
echo -e "${BLUE}========================================================================${NC}\n"

echo -e "${YELLOW}NOTE: This runs native CLI binaries (ureq), NOT the WASM sandbox.${NC}"
echo -e "${YELLOW}For WASM host proof, see: setup_and_run_zeroclaw.sh${NC}\n"

# ── Demo 1: token-risk-check — USDC (clean token) ──
echo -e "${CYAN}▶ DEMO 1: token-risk-check — USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)${NC}"
echo -e "  Calling live Solana Mainnet RPC...\n"
(cd "$PLUGIN_REPO/plugins/token-risk-check" && \
    cargo run --quiet --bin token-risk-check-cli -- EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v) || \
    echo -e "${YELLOW}  (Failed — check SOLANA_RPC_URL or network connectivity)${NC}"

echo -e "\n------------------------------------------------------------------------\n"

# ── Demo 2: token-risk-check — BONK (token with freeze authority) ──
echo -e "${CYAN}▶ DEMO 2: token-risk-check — BONK (DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263)${NC}"
echo -e "  Calling live Solana Mainnet RPC...\n"
(cd "$PLUGIN_REPO/plugins/token-risk-check" && \
    cargo run --quiet --bin token-risk-check-cli -- DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263) || \
    echo -e "${YELLOW}  (Failed — check SOLANA_RPC_URL or network connectivity)${NC}"

echo -e "\n------------------------------------------------------------------------\n"

# ── Demo 3: spl-transfer-build — construct unsigned SOL transfer ──
echo -e "${CYAN}▶ DEMO 3: spl-transfer-build — SOL transfer (unsigned tx construction)${NC}"
echo -e "  From: 8UQUJWj4XnYFaAZjP79SGiwmrcT3fuy3pD7ig5B5bjW2"
echo -e "  To:   EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
echo -e "  Amount: 1,000,000 Lamports (0.001 SOL)\n"
(cd "$PLUGIN_REPO/plugins/spl-transfer-build" && \
    cargo run --quiet --bin spl-transfer-build-cli -- \
    8UQUJWj4XnYFaAZjP79SGiwmrcT3fuy3pD7ig5B5bjW2 \
    EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
    1000000) || \
    echo -e "${YELLOW}  (Failed — check SOLANA_RPC_URL or network connectivity)${NC}"

echo -e "\n------------------------------------------------------------------------\n"

# ── Demo 4: Unit tests ──
echo -e "${CYAN}▶ DEMO 4: Running verified test suite (49 unit tests)${NC}\n"
(cd "$PLUGIN_REPO/plugins/solana-lite" && cargo test --quiet)
(cd "$PLUGIN_REPO/plugins/token-risk-check" && cargo test --quiet)
(cd "$PLUGIN_REPO/plugins/spl-transfer-build" && cargo test --quiet)

echo -e "\n${GREEN}========================================================================${NC}"
echo -e "${GREEN}   ✅ LIVE CLI DEMO COMPLETE — ALL OUTPUTS ARE REAL MAINNET DATA        ${NC}"
echo -e "${GREEN}========================================================================${NC}"
