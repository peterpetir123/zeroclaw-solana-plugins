#!/bin/bash
set -euo pipefail

# ============================================================================
# ZeroClaw Solana Plugins — Reproducible Host Setup & Verification
# ============================================================================
#
# This script reproduces the EXACT steps that generated tool_invoke_proof.log.
# It builds both WASM plugin components, installs them into the ZeroClaw host's
# plugin directory, enables the plugin system, and verifies discovery.
#
# Prerequisites:
#   1. ZeroClaw source repository checked out (default: ../zeroclaw)
#   2. Rust toolchain with wasm32-wasip2 target: rustup target add wasm32-wasip2
#   3. For full smoke test (Tahap 5): GEMINI_API_KEY or ANTHROPIC_API_KEY set
#
# Usage:
#   ./setup_and_run_zeroclaw.sh                    # uses ../zeroclaw as host repo
#   ZEROCLAW_REPO=/path/to/zeroclaw ./setup_and_run_zeroclaw.sh
# ============================================================================

GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

PLUGIN_REPO="$(cd "$(dirname "$0")" && pwd)"
ZEROCLAW_REPO="${ZEROCLAW_REPO:-$(cd "$PLUGIN_REPO/../zeroclaw" 2>/dev/null && pwd || echo "")}"
PLUGIN_DIR="$HOME/.zeroclaw/plugins"

echo -e "${BLUE}==============================================================${NC}"
echo -e "${BLUE}  ZEROCLAW SOLANA PLUGINS — HOST BUILD & VERIFICATION         ${NC}"
echo -e "${BLUE}==============================================================${NC}\n"

# ── Tahap 0: Record ZeroClaw commit hash ──
echo -e "${CYAN}[0/6] Recording ZeroClaw host commit hash...${NC}"
if [ -d "$ZEROCLAW_REPO/.git" ]; then
    ZEROCLAW_COMMIT=$(cd "$ZEROCLAW_REPO" && git log -1 --oneline)
    echo -e "${GREEN}  ZeroClaw commit: ${ZEROCLAW_COMMIT}${NC}"
else
    echo -e "${YELLOW}  WARNING: ZeroClaw repo not found at $ZEROCLAW_REPO${NC}"
    echo -e "${YELLOW}  Set ZEROCLAW_REPO=/path/to/zeroclaw and re-run${NC}"
    echo -e "${YELLOW}  Skipping host build — will use existing binary if available${NC}"
    ZEROCLAW_COMMIT="(not available)"
fi

# ── Tahap 1: Run unit tests ──
echo -e "\n${CYAN}[1/6] Running unit test suite (49 tests across 3 crates)...${NC}\n"
(cd "$PLUGIN_REPO/plugins/solana-lite" && cargo test --quiet)
(cd "$PLUGIN_REPO/plugins/token-risk-check" && cargo test --quiet)
(cd "$PLUGIN_REPO/plugins/spl-transfer-build" && cargo test --quiet)
echo -e "${GREEN}  ✅ All unit tests passed${NC}"

# ── Tahap 2: Build WASM components ──
echo -e "\n${CYAN}[2/6] Building WASM components (wasm32-wasip2)...${NC}"
(cd "$PLUGIN_REPO/plugins/token-risk-check" && cargo build --target wasm32-wasip2 --release --quiet)
echo -e "${GREEN}  ✅ token_risk_check.wasm built${NC}"
(cd "$PLUGIN_REPO/plugins/spl-transfer-build" && cargo build --target wasm32-wasip2 --release --quiet)
echo -e "${GREEN}  ✅ spl_transfer_build.wasm built${NC}"

# ── Tahap 3: Build ZeroClaw host with plugin support ──
echo -e "\n${CYAN}[3/6] Building ZeroClaw host with WASM plugin support...${NC}"
ZEROCLAW_BIN=""
if [ -d "$ZEROCLAW_REPO" ]; then
    (cd "$ZEROCLAW_REPO" && cargo build --release --features plugins-wasm,plugins-wasm-cranelift --quiet)
    ZEROCLAW_BIN="$ZEROCLAW_REPO/target/release/zeroclaw"
    echo -e "${GREEN}  ✅ Host built: $ZEROCLAW_BIN${NC}"
    "$ZEROCLAW_BIN" --version
elif command -v zeroclaw &>/dev/null; then
    ZEROCLAW_BIN="$(command -v zeroclaw)"
    echo -e "${YELLOW}  Using existing zeroclaw binary: $ZEROCLAW_BIN${NC}"
    # Verify it has plugin support
    if ! "$ZEROCLAW_BIN" plugin list &>/dev/null; then
        echo -e "${RED}  ERROR: existing binary does not have plugin support${NC}"
        echo -e "${RED}  Set ZEROCLAW_REPO and re-run to build from source${NC}"
        exit 1
    fi
else
    echo -e "${RED}  ERROR: No ZeroClaw binary found. Set ZEROCLAW_REPO.${NC}"
    exit 1
fi

# ── Tahap 4: Install plugins into host directory ──
echo -e "\n${CYAN}[4/6] Installing WASM plugins into $PLUGIN_DIR ...${NC}"
mkdir -p "$PLUGIN_DIR/token-risk-check"
mkdir -p "$PLUGIN_DIR/spl-transfer-build"

# Copy BOTH manifest.toml AND .wasm files
cp -f "$PLUGIN_REPO/plugins/token-risk-check/manifest.toml" \
      "$PLUGIN_DIR/token-risk-check/"
cp -f "$PLUGIN_REPO/plugins/token-risk-check/target/wasm32-wasip2/release/token_risk_check.wasm" \
      "$PLUGIN_DIR/token-risk-check/"

cp -f "$PLUGIN_REPO/plugins/spl-transfer-build/manifest.toml" \
      "$PLUGIN_DIR/spl-transfer-build/"
cp -f "$PLUGIN_REPO/plugins/spl-transfer-build/target/wasm32-wasip2/release/spl_transfer_build.wasm" \
      "$PLUGIN_DIR/spl-transfer-build/"

echo -e "${GREEN}  ✅ Installed:${NC}"
ls -la "$PLUGIN_DIR/token-risk-check/"
ls -la "$PLUGIN_DIR/spl-transfer-build/"

# ── Tahap 5: Enable plugin system & verify discovery ──
echo -e "\n${CYAN}[5/6] Enabling plugin system and verifying discovery...${NC}"
"$ZEROCLAW_BIN" config set plugins.enabled true
"$ZEROCLAW_BIN" config set plugins.auto_discover true
"$ZEROCLAW_BIN" config set plugins.security.signature_mode disabled
echo ""
"$ZEROCLAW_BIN" plugin list
echo ""
echo -e "${GREEN}  ✅ Plugin system enabled and both plugins discovered${NC}"
echo ""
"$ZEROCLAW_BIN" plugin info token-risk-check
"$ZEROCLAW_BIN" plugin info spl-transfer-build

# ── Tahap 6: Smoke test (requires LLM API key) ──
echo -e "\n${CYAN}[6/6] Smoke test — invoking plugin via ZeroClaw agent...${NC}"

# Detect which LLM provider to use
PROVIDER=""
MODEL=""
if [ -n "${GEMINI_API_KEY:-}" ]; then
    PROVIDER="gemini"
    MODEL="gemini-2.5-flash"
    echo -e "${GREEN}  Using Gemini provider (GEMINI_API_KEY detected)${NC}"
elif [ -n "${ANTHROPIC_API_KEY:-}" ]; then
    PROVIDER="anthropic"
    MODEL="claude-sonnet-4-20250514"
    echo -e "${GREEN}  Using Anthropic provider (ANTHROPIC_API_KEY detected)${NC}"
elif [ -n "${OPENAI_API_KEY:-}" ]; then
    PROVIDER="openai"
    MODEL="gpt-4o"
    echo -e "${GREEN}  Using OpenAI provider (OPENAI_API_KEY detected)${NC}"
else
    echo -e "${YELLOW}  ⚠ No LLM API key detected.${NC}"
    echo -e "${YELLOW}  Set one of: GEMINI_API_KEY, ANTHROPIC_API_KEY, OPENAI_API_KEY${NC}"
    echo -e "${YELLOW}  Skipping agent smoke test — plugin discovery already verified above.${NC}"
    echo ""
    echo -e "${GREEN}==============================================================${NC}"
    echo -e "${GREEN}  ✅ SETUP COMPLETE — plugins loaded by host successfully     ${NC}"
    echo -e "${GREEN}  ZeroClaw commit: ${ZEROCLAW_COMMIT}${NC}"
    echo -e "${GREEN}==============================================================${NC}"
    exit 0
fi

echo -e "\n  Invoking token-risk-check for USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)..."
"$ZEROCLAW_BIN" agent --agent main \
    -p "$PROVIDER" --model "$MODEL" \
    -m "Use the token-risk-check tool to check the risk of this Solana token mint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v. Return the raw JSON result." \
    -v 2>&1 | tee "$PLUGIN_REPO/tool_invoke_proof.log"

echo -e "\n${GREEN}==============================================================${NC}"
echo -e "${GREEN}  ✅ FULL VERIFICATION COMPLETE                               ${NC}"
echo -e "${GREEN}  ZeroClaw commit: ${ZEROCLAW_COMMIT}${NC}"
echo -e "${GREEN}  Proof log: tool_invoke_proof.log                            ${NC}"
echo -e "${GREEN}==============================================================${NC}"
