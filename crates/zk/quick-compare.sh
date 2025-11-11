#!/bin/bash
# Quick Backend Comparison Script
#
# Fast comparison with minimal sampling (not statistically rigorous, but gives rough idea)
#
# Usage: ./quick-compare.sh

echo "🚀 Quick ZK Backend Comparison"
echo "================================"
echo ""
echo "⚠️  Note: Using minimal sampling (10 samples) for speed"
echo "    For rigorous benchmarks, use ./compare-backends.sh"
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

echo -e "${BLUE}Testing RISC0 (zkVM)...${NC}"
echo ""

# Run RISC0 with minimal sampling
RISC0_SKIP_BUILD=1 cargo bench --no-default-features --features risc0 \
    --bench backend_comparison -- --sample-size 10 --warm-up-time 1 \
    prove_move_1_actors 2>&1 | grep -E "time:" || echo "  (Benchmark output above)"

echo ""
echo -e "${GREEN}✓${NC} RISC0 complete"
echo ""

echo -e "${BLUE}Testing Arkworks (R1CS)...${NC}"
echo ""

# Run Arkworks with minimal sampling
cargo bench --no-default-features --features arkworks \
    --bench backend_comparison -- --sample-size 10 --warm-up-time 1 \
    prove_move_1_actors 2>&1 | grep -E "time:" || echo "  (Benchmark output above)"

echo ""
echo -e "${GREEN}✓${NC} Arkworks complete"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${YELLOW}📊 Quick Comparison Results${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Results shown above for 1-actor Move action proof generation"
echo ""
echo -e "${YELLOW}Key Comparison: zkVM (RISC0) vs R1CS (Arkworks)${NC}"
echo "  • RISC0: General-purpose zkVM, easier development, slower proving"
echo "  • Arkworks: Hand-crafted R1CS circuits, faster proving ✅"
echo ""
echo "Expected performance characteristics:"
echo "  • RISC0: ~5-10 seconds per proof (zkVM execution + proof generation)"
echo "  • Arkworks: ~1-2 seconds per proof (with cached keys)"
echo ""
echo "Note: Arkworks benchmarks now use cached Groth16 keys (generated once"
echo "      at ~15-18 seconds during setup). This reflects production usage"
echo "      where keys are pre-generated and reused for all proofs."
echo ""
echo -e "${GREEN}✓ Quick comparison complete!${NC}"
echo ""
