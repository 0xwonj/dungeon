# Justfile for Dungeon - ZK-provable RPG
#
# This file provides convenient commands for working with multiple ZK backends.
# Just is a modern command runner (like Make, but better).
#
# Installation:
#   cargo install just
#
# Usage:
#   just <command> [backend]
#
# Examples:
#   just build              # Build with default backend (risc0)
#   just build stub         # Build with stub backend
#   just run-fast stub      # Run in fast mode (no proofs, no persistence)
#   just test stub          # Test with stub backend
#   just lint               # Lint with default backend
#
# Environment Variables:
#   ZK_BACKEND - Set default backend (risc0, stub, sp1, arkworks)
#
# Available Backends:
#   risc0       - RISC0 zkVM (production, real proofs, slow)
#   stub        - Stub prover (instant, no real proofs, testing only)
#   sp1         - SP1 zkVM (proof mode via SP1_PROOF_MODE env var)
#   arkworks    - Arkworks circuits (not implemented yet)

# ============================================================================
# Configuration & Settings
# ============================================================================

# Use bash for all recipe scripts
set shell := ["bash", "-c"]

# Enable .env file loading
set dotenv-load := true

# Use positional arguments ($1, $2) instead of interpolation for safety
set positional-arguments := true

# Prevent duplicate recipe definitions
set allow-duplicate-recipes := false

# Export all variables as environment variables
set export := true

# Default backend from environment variable or fallback to risc0
default_backend := env_var_or_default('ZK_BACKEND', 'risc0')

# ============================================================================
# Help & Info
# ============================================================================

# Show all available commands (default recipe)
@default:
    just --list

# Show detailed help with examples
help:
    @echo "╔════════════════════════════════════════════════════════════════╗"
    @echo "║         Dungeon - ZK-Provable RPG Development Commands         ║"
    @echo "╚════════════════════════════════════════════════════════════════╝"
    @echo ""
    @echo "Available backends:"
    @echo "  risc0       Production RISC0 zkVM (real proofs, slow)"
    @echo "  stub        Dummy prover (instant, testing only)"
    @echo "  sp1         SP1 zkVM (use SP1_PROOF_MODE for proof type)"
    @echo "  arkworks    Arkworks circuits (not implemented)"
    @echo ""
    @echo "SP1 Proof Modes (SP1_PROOF_MODE):"
    @echo "  compressed  Compressed STARK (~4-5MB, off-chain) [default]"
    @echo "  groth16     Groth16 SNARK (~260 bytes, on-chain)"
    @echo "  plonk       PLONK SNARK (~868 bytes, on-chain)"
    @echo ""
    @echo "Common workflows:"
    @echo "  just build stub          Fast development build"
    @echo "  just run-fast stub       Run in fast mode (no proofs, no persistence)"
    @echo "  just test stub           Fast tests"
    @echo "  just lint                Check code quality"
    @echo "  just check-all           Verify all backends compile"
    @echo ""
    @echo "Development tools:"
    @echo "  just tail-logs           Monitor latest session logs"
    @echo "  just tail-logs <id>      Monitor specific session"
    @echo "  just read-state <nonce>  Read and inspect state file"
    @echo "  just read-actions        Read and inspect action log (latest session)"
    @echo "  just clean-data          Clean all data (with confirmation)"
    @echo "  just clean-logs          Clean only logs"
    @echo "  just rebuild-guest       Rebuild guest program (fixes malformed binary)"
    @echo ""
    @echo "Quick aliases (frequently used):"
    @echo "  b     Build workspace            (just build)"
    @echo "  r     Run CLI client             (just run)"
    @echo "  t     Run tests                  (just test)"
    @echo "  l     Run clippy lints           (just lint)"
    @echo "  f     Format code                (just fmt)"
    @echo "  fl    Format + lint              (just fmt-lint)"
    @echo "  c     Run all checks             (just check)"
    @echo ""
    @echo "Set default backend:"
    @echo "  export ZK_BACKEND=stub"
    @echo "  just build               # Uses stub automatically"
    @echo ""
    @echo "Current backend: {{default_backend}}"

# Show current backend configuration
info:
    @echo "Current Configuration:"
    @echo "  Backend: {{default_backend}}"
    @echo ""
    @echo "Environment Variables:"
    @echo "  ZK_BACKEND=${ZK_BACKEND:-not set}"
    @echo "  RISC0_SKIP_BUILD=${RISC0_SKIP_BUILD:-not set}"
    @echo "  RISC0_DEV_MODE=${RISC0_DEV_MODE:-not set}"

# ============================================================================
# Build Commands
# ============================================================================

# Build workspace with specified backend
build backend=default_backend:
    @just _exec {{backend}} build --workspace

alias b := build

# Build specific package with specified backend
build-package package backend=default_backend:
    @just _exec {{backend}} build -p {{package}}

# Build in release mode with specified backend
build-release backend=default_backend:
    @just _exec {{backend}} build --workspace --release

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean

# Build only the ZK guest program (RISC0 only)
build-guest:
    @echo "🔨 Building RISC0 guest program..."
    cargo build -p zk

# ============================================================================
# Run Commands
# ============================================================================

# Run CLI client with specified backend
run backend=default_backend *args='':
    @just _exec {{backend}} run -p dungeon-client --no-default-features --features "frontend-cli,zkvm-{{backend}}" {{args}}

alias r := run

# Run CLI in fast mode (no proof generation, no persistence)
run-fast backend=default_backend *args='':
    #!/usr/bin/env bash
    set -euo pipefail
    export ENABLE_ZK_PROVING=false
    export ENABLE_PERSISTENCE=false
    backend="$1"
    shift
    just _exec "$backend" run -p dungeon-client --no-default-features --features "frontend-cli,zkvm-$backend" "$@"

# Run CLI in release mode
run-release backend=default_backend *args='':
    @just _exec {{backend}} run -p dungeon-client --no-default-features --features "frontend-cli,zkvm-{{backend}}" --release {{args}}

# Run CLI with Sui blockchain integration
run-sui backend=default_backend *args='':
    @just _exec {{backend}} run -p dungeon-client --no-default-features --features "frontend-cli,blockchain-sui,zkvm-{{backend}}" {{args}}

alias rs := run-sui

# ============================================================================
# Test Commands
# ============================================================================

# Run all tests with specified backend
test backend=default_backend *args='':
    @just _exec {{backend}} test --workspace {{args}}

alias t := test

# Run tests for specific package
test-package package backend=default_backend *args='':
    @just _exec {{backend}} test -p {{package}} {{args}}

# Run integration tests only
test-integration backend=default_backend:
    @just _exec {{backend}} test --workspace --test '*'

# Run lib tests only
test-lib backend=default_backend:
    @just _exec {{backend}} test --workspace --lib

# Run tests with output visible (nocapture)
test-verbose backend=default_backend:
    @just _exec {{backend}} test --workspace -- --nocapture

# Watch tests and re-run on file changes (requires cargo-watch)
watch-test backend=default_backend:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-watch &> /dev/null; then
        echo "⚠️  cargo-watch not found. Installing..."
        cargo install cargo-watch
    fi
    echo "👀 Watching for changes and running tests..."
    case "$1" in
        risc0)
            cargo watch -x "test --workspace"
            ;;
        stub)
            cargo watch -x "test --workspace --no-default-features --features stub"
            ;;
        *)
            echo "❌ Backend '$1' not supported for watch mode"
            exit 1
            ;;
    esac

# ============================================================================
# Code Quality Commands
# ============================================================================

# Run clippy lints with specified backend
lint backend=default_backend:
    @just _exec {{backend}} clippy --workspace --all-targets -- -D warnings

alias l := lint

# Run clippy with automatic fixes
lint-fix backend=default_backend:
    @just _exec {{backend}} clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# Format all code
fmt:
    @echo "🎨 Formatting code..."
    cargo fmt --all

alias f := fmt

# Check formatting without modifying files
fmt-check:
    @echo "🔍 Checking code formatting..."
    cargo fmt --all --check

# Format code and run lints together
fmt-lint backend=default_backend: fmt (lint backend)
    @echo "✅ Format and lint complete for {{backend}} backend!"

alias fl := fmt-lint

# Run all checks (format, clippy, tests)
check backend=default_backend: fmt-check (lint backend) (test backend)
    @echo "✅ All checks passed for {{backend}} backend!"

alias c := check

# ============================================================================
# Multi-Backend Verification
# ============================================================================

# Verify all implemented backends compile (CI/CD use)
check-all:
    @echo "🔍 Verifying all implemented backends compile..."
    @echo ""
    @echo "Checking risc0 backend..."
    @just _exec risc0 check --workspace
    @echo "✅ RISC0 verified"
    @echo ""
    @echo "Checking stub backend..."
    @just _exec stub check --workspace
    @echo "✅ Stub verified"
    @echo ""
    @echo "✅ All implemented backends verified!"
    @echo ""
    @echo "Note: SP1 and Arkworks backends are not yet implemented"

# Lint all backends (comprehensive check)
lint-all: (lint "risc0") (lint "stub")
    @echo "✅ All backends linted!"

# ============================================================================
# Documentation
# ============================================================================

# Generate and open documentation
doc backend=default_backend:
    @just _exec {{backend}} doc --workspace --no-deps --open

# Generate documentation without opening
doc-build backend=default_backend:
    @just _exec {{backend}} doc --workspace --no-deps

# ============================================================================
# Development Workflows
# ============================================================================

# Fast development loop: format, lint, test (stub backend)
dev: fmt (lint "stub") (test "stub")
    @echo "✅ Development checks passed!"

# Pre-commit checks (recommended before committing)
pre-commit: fmt (lint "stub") (test "stub")
    @echo "✅ Pre-commit checks passed!"

# Full CI simulation (what CI runs)
ci: fmt-check check-all (test "stub")
    @echo "✅ CI simulation passed!"

# Watch mode for continuous development (format + test on changes)
watch backend=default_backend:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-watch &> /dev/null; then
        echo "⚠️  cargo-watch not found. Installing..."
        cargo install cargo-watch
    fi
    echo "👀 Watching for changes (format + test)..."
    case "$1" in
        risc0)
            cargo watch -x fmt -x "test --workspace"
            ;;
        stub)
            cargo watch -x fmt -x "test --workspace --no-default-features --features stub"
            ;;
        *)
            echo "❌ Backend '$1' not supported for watch mode"
            exit 1
            ;;
    esac

# ============================================================================
# Benchmarking & Performance
# ============================================================================

# Run benchmarks (when implemented)
bench backend=default_backend:
    @just _exec {{backend}} bench --workspace

# ============================================================================
# Utility Commands
# ============================================================================

# Show dependency tree for a package
tree package='':
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "$1" ]; then
        cargo tree --workspace
    else
        cargo tree -p "$1"
    fi

# Update dependencies
update:
    @echo "📦 Updating dependencies..."
    cargo update

# Check for outdated dependencies
outdated:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-outdated &> /dev/null; then
        echo "⚠️  cargo-outdated not found. Installing..."
        cargo install cargo-outdated
    fi
    echo "🔍 Checking for outdated dependencies..."
    cargo outdated

# ============================================================================
# Development Tools (xtask)
# ============================================================================

# Monitor client logs in real-time (optionally specify session)
tail-logs session='':
    @cargo run -q -p xtask -- tail-logs {{session}}

# Clean save data and logs
clean-data *args='':
    @cargo run -q -p xtask -- clean {{args}}

# Clean only logs (faster than clean-data --logs)
clean-logs:
    @cargo run -q -p xtask -- clean --logs

# Rebuild RISC0 guest program (fixes malformed binary errors)
rebuild-guest:
    @echo "🧹 Cleaning zk crate..."
    @cargo clean -p zk
    @echo "✅ Cleaned zk crate"
    @echo "🔨 Building guest program (this may take a minute)..."
    @cargo build -p zk
    @echo "✅ Guest program rebuilt successfully"
    @echo ""
    @echo "💡 You can now run the client without malformed binary errors:"
    @echo "   cargo run -p client-cli"
    @echo "   or"
    @echo "   just run risc0"

# List all available sessions
sessions:
    @echo "📋 Available sessions:"
    @ls -1t "$(cargo run -q -p xtask -- tail-logs --help 2>&1 | grep -o '/.*logs' | head -1 || echo "$HOME/Library/Caches/dungeon/logs")" 2>/dev/null || echo "  No sessions found"

# Read and inspect state file at specified nonce
read-state nonce format='summary':
    @cargo run -q -p xtask -- read-state {{nonce}} --format {{format}}

# Read and inspect action log for a session (default: latest)
read-actions nonce='' *args='':
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "$1" ]; then
        cargo run -q -p xtask -- read-actions ${@:2}
    else
        cargo run -q -p xtask -- read-actions "$1" ${@:2}
    fi

# ============================================================================
# Internal Helpers (Private Recipes)
# ============================================================================

# Execute cargo command with appropriate backend configuration
[private]
_exec backend *args:
    #!/usr/bin/env bash
    set -euo pipefail

    backend_name="$1"
    shift

    # Separate cargo args before and after '--'
    cargo_args=()
    rustc_args=()
    found_separator=false

    for arg in "$@"; do
        if [ "$arg" = "--" ]; then
            found_separator=true
        elif [ "$found_separator" = true ]; then
            rustc_args+=("$arg")
        else
            cargo_args+=("$arg")
        fi
    done

    case "$backend_name" in
        risc0)
            echo "🔧 Using RISC0 backend (production mode)"
            if [ "$found_separator" = true ]; then
                cargo "${cargo_args[@]}" -- "${rustc_args[@]}"
            else
                cargo "${cargo_args[@]}"
            fi
            ;;
        stub)
            echo "🎭 Using Stub backend (no real proofs)"
            if [ "$found_separator" = true ]; then
                cargo "${cargo_args[@]}" --no-default-features --features stub -- "${rustc_args[@]}"
            else
                cargo "${cargo_args[@]}" --no-default-features --features stub
            fi
            ;;
        sp1)
            proof_mode="${SP1_PROOF_MODE:-compressed}"
            echo "🔧 Using SP1 backend (proof mode: $proof_mode)"
            if [ "$found_separator" = true ]; then
                cargo "${cargo_args[@]}" --no-default-features --features sp1 -- "${rustc_args[@]}"
            else
                cargo "${cargo_args[@]}" --no-default-features --features sp1
            fi
            ;;
        arkworks)
            echo "🔧 Using Arkworks backend"
            if [ "$found_separator" = true ]; then
                cargo "${cargo_args[@]}" --no-default-features --features arkworks -- "${rustc_args[@]}"
            else
                cargo "${cargo_args[@]}" --no-default-features --features arkworks
            fi
            ;;
        *)
            echo "❌ Error: Unknown backend '$backend_name'"
            echo ""
            echo "Available backends:"
            echo "  risc0, stub, sp1, arkworks"
            echo ""
            echo "For SP1, use SP1_PROOF_MODE to select proof type:"
            echo "  SP1_PROOF_MODE=compressed (default)"
            echo "  SP1_PROOF_MODE=groth16"
            echo "  SP1_PROOF_MODE=plonk"
            exit 1
            ;;
    esac

# Install common development tools
[private]
install-dev-tools:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "🔧 Installing common development tools..."
    tools=(
        "cargo-watch"
        "cargo-outdated"
    )

    for tool in "${tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            echo "  Installing $tool..."
            cargo install "$tool"
        else
            echo "  ✓ $tool already installed"
        fi
    done

    echo "✅ All development tools installed!"

# Bootstrap development environment
bootstrap: install-dev-tools
    @echo "🚀 Development environment ready!"
    @echo ""
    @echo "Try these commands:"
    @echo "  just dev          # Fast development loop"
    @echo "  just watch stub   # Watch mode with auto-testing"
    @echo "  just help         # See all available commands"
