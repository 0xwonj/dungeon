#!/usr/bin/env bash
# Build RISC0 zkVM guest program
#
# Usage:
#   ./scripts/build-guest.sh [--release]
#
# This script builds only the guest program without rebuilding the entire workspace.
# Useful for iterating on zkVM guest code.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Parse arguments
BUILD_MODE="debug"
if [[ "$1" == "--release" ]]; then
    BUILD_MODE="release"
fi

echo "Building RISC0 guest program (mode: $BUILD_MODE)..."

cd "$PROJECT_ROOT"

if [[ "$BUILD_MODE" == "release" ]]; then
    cargo build -p zk --release --features risc0
else
    cargo build -p zk --features risc0
fi

echo ""
echo "✓ Guest program built successfully"
echo ""
echo "Binary location:"
# RISC0 always builds guest in release mode for optimization
GUEST_PATH="$PROJECT_ROOT/target/riscv-guest/zk/game-verifier/riscv32im-risc0-zkvm-elf/release/game-verifier.bin"
if [[ -f "$GUEST_PATH" ]]; then
    ls -lh "$GUEST_PATH"
else
    echo "  (Binary not found at expected location: $GUEST_PATH)"
fi
