#!/bin/bash
# Backend Comparison Script
#
# This script runs benchmarks for both RISC0 and Arkworks backends
# and generates a comparison report.
#
# Usage: ./compare-backends.sh

set -e

echo "🔬 ZK Backend Comparison Tool"
echo "=============================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Must run from crates/zk directory"
    exit 1
fi

# Step 1: Run RISC0 benchmarks
echo -e "${BLUE}Step 1/4:${NC} Running RISC0 benchmarks (saving baseline)..."
echo ""
RISC0_SKIP_BUILD=1 cargo bench --no-default-features --features risc0 \
    --bench backend_comparison -- --save-baseline risc0 2>&1 | grep -E "Benchmarking|time:|📊"

echo ""
echo -e "${GREEN}✓${NC} RISC0 baseline saved"
echo ""

# Step 2: Run Arkworks benchmarks for comparison
echo -e "${BLUE}Step 2/3:${NC} Running Arkworks (R1CS) benchmarks (comparing to baseline)..."
echo ""
cargo bench --no-default-features --features arkworks \
    --bench backend_comparison -- --baseline risc0 2>&1 | grep -E "Benchmarking|time:|change:|📊"

echo ""
echo -e "${GREEN}✓${NC} Arkworks comparison complete"
echo ""

# Step 3: Summary
echo -e "${BLUE}Step 3/3:${NC} Generating summary..."
echo ""
cargo bench --no-default-features --features arkworks \
    --bench backend_comparison -- --save-baseline arkworks 2>&1 | grep -E "Benchmarking|time:|📊"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${YELLOW}📊 Benchmark Results Summary${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "View detailed HTML reports:"
echo "  • RISC0:     target/criterion/backend_comparison/risc0/report/index.html"
echo "  • Arkworks:  target/criterion/backend_comparison/arkworks/report/index.html"
echo ""
echo "Comparison report:"
echo "  • open target/criterion/backend_comparison/report/index.html"
echo ""
echo -e "${YELLOW}Key Comparison: zkVM (RISC0) vs R1CS (Arkworks)${NC}"
echo "  • RISC0: General-purpose zkVM, easier development, slower proving"
echo "  • Arkworks: Hand-crafted R1CS circuits, faster proving, more complex development"
echo ""
echo -e "${GREEN}✓ Comparison complete!${NC}"
echo ""
