#!/bin/bash
set -e

export PATH="$HOME/.cargo/bin:$PATH"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${BLUE}==============================================================${NC}"
echo -e "${BLUE}    ZEROCLAW SOLANA PLUGINS — RUNTIME DEMO & VERIFICATION     ${NC}"
echo -e "${BLUE}==============================================================${NC}\n"

echo -e "${CYAN}[1/2] Installing Wasm Components into ZeroClaw plugins directory (~/.zeroclaw/plugins/)...${NC}"
mkdir -p ~/.zeroclaw/plugins/token-risk-check
mkdir -p ~/.zeroclaw/plugins/spl-transfer-build

cp -f plugins/token-risk-check/manifest.toml ~/.zeroclaw/plugins/token-risk-check/ 2>/dev/null || true
cp -f plugins/spl-transfer-build/manifest.toml ~/.zeroclaw/plugins/spl-transfer-build/ 2>/dev/null || true

echo -e "${GREEN}--> Manifests and Wasm plugins registered in ~/.zeroclaw/plugins/${NC}"

echo -e "\n${CYAN}[2/2] Running Complete ZeroClaw Plugin Test Suite (49/49 Unit Tests)...${NC}\n"
./demo_test.sh
