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
#   sp1         - SP1 zkVM (not implemented yet)
#   arkworks    - Arkworks circuits (not implemented yet)

# ============================================================================
# Configuration
# ============================================================================

# Use bash for all recipe scripts
set shell := ["bash", "-c"]

# Enable .env file loading (optional)
set dotenv-load := true

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
    @echo "  sp1         SP1 zkVM (not implemented)"
    @echo "  arkworks    Arkworks circuits (not implemented)"
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
    @echo "  just clean-data          Clean all data (with confirmation)"
    @echo "  just clean-logs          Clean only logs"
    @echo "  just rebuild-guest       Rebuild guest program (fixes malformed binary)"
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

# Build workspace with specified backend (default: from ZK_BACKEND env or risc0)
build backend=default_backend:
    @just _exec {{backend}} "build --workspace"

# Build specific package with specified backend
build-package package backend=default_backend:
    @just _exec {{backend}} "build -p {{package}}"

# Build in release mode with specified backend
build-release backend=default_backend:
    @just _exec {{backend}} "build --workspace --release"

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
    @just _exec {{backend}} "run -p client-cli {{args}}"

# Run CLI in fast mode (no proof generation, no persistence)
run-fast backend=default_backend *args='':
    #!/usr/bin/env bash
    export ENABLE_ZK_PROVING=false
    export ENABLE_PERSISTENCE=false
    just _exec {{backend}} "run -p client-cli {{args}}"

# Run CLI in release mode
run-release backend=default_backend *args='':
    @just _exec {{backend}} "run -p client-cli --release {{args}}"

# ============================================================================
# Test Commands
# ============================================================================

# Run all tests with specified backend
test backend=default_backend *args='':
    @just _exec {{backend}} "test --workspace {{args}}"

# Run tests for specific package
test-package package backend=default_backend *args='':
    @just _exec {{backend}} "test -p {{package}} {{args}}"

# Run integration tests only
test-integration backend=default_backend:
    @just _exec {{backend}} "test --workspace --test '*'"

# Run lib tests only
test-lib backend=default_backend:
    @just _exec {{backend}} "test --workspace --lib"

# Run tests with output visible (nocapture)
test-verbose backend=default_backend:
    @just _exec {{backend}} "test --workspace -- --nocapture"

# ============================================================================
# Code Quality Commands
# ============================================================================

# Run clippy lints with specified backend
lint backend=default_backend:
    @just _exec {{backend}} "clippy --workspace --all-targets"

# Run clippy with automatic fixes
lint-fix backend=default_backend:
    @just _exec {{backend}} "clippy --workspace --all-targets --fix --allow-dirty"

# Format all code
fmt:
    @echo "🎨 Formatting code..."
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    @echo "🔍 Checking code formatting..."
    cargo fmt --all --check

# Run all checks (format, clippy, tests)
check backend=default_backend:
    @echo "🔍 Running all checks with {{backend}} backend..."
    @just fmt-check
    @just lint {{backend}}
    @just test {{backend}}

# ============================================================================
# Multi-Backend Verification
# ============================================================================

# Verify all implemented backends compile (CI/CD use)
check-all:
    @echo "🔍 Verifying all implemented backends compile..."
    @echo ""
    @echo "Checking risc0 backend..."
    @just _exec risc0 "check --workspace"
    @echo "✅ RISC0 verified"
    @echo ""
    @echo "Checking stub backend..."
    @just _exec stub "check --workspace"
    @echo "✅ Stub verified"
    @echo ""
    @echo "✅ All implemented backends verified!"
    @echo ""
    @echo "Note: SP1 and Arkworks backends are not yet implemented"

# Lint all backends (comprehensive check)
lint-all:
    @echo "🔍 Linting all backends..."
    @just lint risc0
    @just lint stub
    @echo "✅ All backends linted!"

# ============================================================================
# Documentation
# ============================================================================

# Generate and open documentation
doc backend=default_backend:
    @just _exec {{backend}} "doc --workspace --no-deps --open"

# Generate documentation without opening
doc-build backend=default_backend:
    @just _exec {{backend}} "doc --workspace --no-deps"

# ============================================================================
# Development Workflows
# ============================================================================

# Fast development loop: format, lint, test (stub backend)
dev:
    @echo "🚀 Running fast development loop (stub backend)..."
    @just fmt
    @just lint stub
    @just test stub

# Pre-commit checks (recommended before committing)
pre-commit:
    @echo "🔍 Running pre-commit checks..."
    @just fmt
    @just lint stub
    @just test stub
    @echo "✅ Pre-commit checks passed!"

# Full CI simulation (what CI runs)
ci:
    @echo "🤖 Running full CI simulation..."
    @just fmt-check
    @just check-all
    @just test stub
    @echo "✅ CI simulation passed!"

# ============================================================================
# Benchmarking & Performance
# ============================================================================

# Run benchmarks (when implemented)
bench backend=default_backend:
    @just _exec {{backend}} "bench --workspace"

# ============================================================================
# Utility Commands
# ============================================================================

# Show dependency tree for a package
tree package='':
    @if [ -z "{{package}}" ]; then \
        cargo tree --workspace; \
    else \
        cargo tree -p {{package}}; \
    fi

# Update dependencies
update:
    @echo "📦 Updating dependencies..."
    cargo update

# Check for outdated dependencies
outdated:
    @echo "🔍 Checking for outdated dependencies..."
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

# ============================================================================
# Internal Helpers (Private Recipes)
# ============================================================================

# Execute cargo command with appropriate backend configuration
[private]
_exec backend *args:
    #!/usr/bin/env bash
    set -euo pipefail

    case "{{backend}}" in
        risc0)
            echo "🔧 Using RISC0 backend (production mode)"
            cargo {{args}}
            ;;
        stub)
            echo "🎭 Using Stub backend (no real proofs)"
            cargo {{args}} --no-default-features --features stub
            ;;
        sp1)
            echo "🔧 Using SP1 backend"
            cargo {{args}} --no-default-features --features sp1
            ;;
        arkworks)
            echo "🔧 Using Arkworks backend"
            cargo {{args}} --no-default-features --features arkworks
            ;;
        *)
            echo "❌ Error: Unknown backend '{{backend}}'"
            echo ""
            echo "Available backends:"
            echo "  risc0, stub, sp1, arkworks"
            exit 1
            ;;
    esac
