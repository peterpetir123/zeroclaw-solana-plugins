#!/bin/bash
set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}==============================================================${NC}"
echo -e "${BLUE}   ZEROCLAW SOLANA PLUGIN SUITE — COMPREHENSIVE DEMO & TEST  ${NC}"
echo -e "${BLUE}==============================================================${NC}\n"

echo -e "${CYAN}[1/3] Testing solana-lite (Shared Core & Cryptography)...${NC}"
cd plugins/solana-lite
cargo test
cd ../..

echo -e "\n${CYAN}[2/3] Testing token-risk-check (T0 Security Auditor)...${NC}"
cd plugins/token-risk-check
cargo test
echo -e "${GREEN}--> Building Wasm Component (wasm32-wasip2)...${NC}"
cargo build --target wasm32-wasip2 --release
cd ../..

echo -e "\n${CYAN}[3/3] Testing spl-transfer-build (T1 Unsigned Transaction Builder)...${NC}"
cd plugins/spl-transfer-build
cargo test
echo -e "${GREEN}--> Building Wasm Component (wasm32-wasip2)...${NC}"
cargo build --target wasm32-wasip2 --release
cd ../..

echo -e "\n${GREEN}==============================================================${NC}"
echo -e "${GREEN}   ✅ ALL 49 UNIT TESTS PASSED & WASM COMPONENTS BUILT!     ${NC}"
echo -e "${GREEN}==============================================================${NC}"
