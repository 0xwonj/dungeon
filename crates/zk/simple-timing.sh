#!/bin/bash
# Simple timing comparison - just time a single proof for each backend
#
# Usage: ./simple-timing.sh

set -e

echo "🚀 Simple Backend Timing Comparison"
echo "====================================="
echo ""
echo "Timing 1 proof per backend (not statistically rigorous)"
echo ""

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Must run from crates/zk directory"
    exit 1
fi

echo -e "${BLUE}Step 1: Timing RISC0 proof generation...${NC}"
echo ""
RISC0_SKIP_BUILD=1 time -p cargo run --example risc0_simple_proof --no-default-features --features risc0 --release 2>&1 | grep -E "^real|^user|^sys" || true
echo ""
echo -e "${GREEN}✓${NC} RISC0 timed"
echo ""

echo -e "${BLUE}Step 2: Timing Arkworks proof generation...${NC}"
echo ""
time -p cargo run --example arkworks_simple_proof --no-default-features --features arkworks --release 2>&1 | grep -E "^real|^user|^sys" || true
echo ""
echo -e "${GREEN}✓${NC} Arkworks timed"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${YELLOW}Comparison Summary${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Times shown above are for generating 1 Move action proof"
echo ""
echo -e "${YELLOW}Expected results:${NC}"
echo "  • RISC0: ~5-10 seconds (general-purpose zkVM)"
echo "  • Arkworks: ~15-20 seconds (includes Groth16 key generation)"
echo ""
echo "Note: Arkworks would be faster with pre-generated keys"
echo ""
