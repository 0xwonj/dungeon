# Client Restructuring Implementation Checklist

This is the step-by-step execution checklist for implementing the client restructuring plan.
Each task should be checked off as completed.

## Pre-Flight Checks

- [ ] Review [client-restructuring-plan.md](./client-restructuring-plan.md)
- [ ] Backup current working branch: `git checkout -b backup/pre-restructuring`
- [ ] Create feature branch: `git checkout -b refactor/client-restructuring`
- [ ] Ensure clean working directory: `git status`
- [ ] Run existing tests: `just test stub`
- [ ] Document current build commands for comparison

---

## Phase 1: Directory Restructuring (1-2 hours)

### 1.1 Create Directory Structure

```bash
cd crates/client
mkdir -p frontend blockchain src
```

- [ ] Create `frontend/` directory
- [ ] Create `blockchain/` directory
- [ ] Create `src/` directory for main binary

### 1.2 Move Existing Crates

```bash
# Move frontend crates
mv core frontend/
mv cli frontend/

# Move blockchain crates
mv sui blockchain/

# Verify structure
ls -la frontend/
ls -la blockchain/
```

- [ ] Move `core/` → `frontend/core/`
- [ ] Move `cli/` → `frontend/cli/`
- [ ] Move `sui/` → `blockchain/sui/`
- [ ] Move `bootstrap/` → stays at `client/bootstrap/` (top-level)

### 1.3 Create Placeholder Files

```bash
# Create main binary files
touch Cargo.toml
touch src/main.rs

# Create blockchain-core crate
mkdir -p blockchain/core/src
touch blockchain/core/Cargo.toml
touch blockchain/core/src/lib.rs
touch blockchain/core/src/traits.rs
touch blockchain/core/src/types.rs
touch blockchain/core/src/mock.rs
```

- [ ] Create `crates/client/Cargo.toml`
- [ ] Create `crates/client/src/main.rs`
- [ ] Create `crates/client/blockchain/core/` structure

### 1.4 Verify Structure

```bash
tree -L 3 crates/client/
```

Expected output:
```
crates/client/
├── Cargo.toml (new)
├── src/
│   └── main.rs (new)
├── bootstrap/
│   ├── Cargo.toml
│   └── src/
├── frontend/
│   ├── core/
│   │   ├── Cargo.toml
│   │   └── src/
│   └── cli/
│       ├── Cargo.toml
│       └── src/
└── blockchain/
    ├── core/ (new)
    │   ├── Cargo.toml
    │   └── src/
    └── sui/
        ├── Cargo.toml
        └── src/
```

- [ ] Directory structure matches expected layout
- [ ] No files lost during move

### 1.5 Update Workspace Cargo.toml

Edit `/home/wonjae/code/dungeon/Cargo.toml`:

```toml
[workspace]
members = [
    "crates/game/core",
    "crates/game/content",
    "crates/runtime",
    "crates/zk",

    # Client umbrella (binary crate)
    "crates/client",

    # Client sub-crates
    "crates/client/bootstrap",
    "crates/client/frontend/core",
    "crates/client/frontend/cli",
    "crates/client/blockchain/core",
    "crates/client/blockchain/sui",

    "crates/xtask",
    "crates/behavior-tree",
]

[workspace.dependencies]
# ... existing dependencies ...

# Update client crate paths
client-bootstrap = { path = "crates/client/bootstrap", default-features = false }
client-frontend-core = { path = "crates/client/frontend/core" }
client-frontend-cli = { path = "crates/client/frontend/cli" }
client-blockchain-core = { path = "crates/client/blockchain/core" }
client-blockchain-sui = { path = "crates/client/blockchain/sui" }
```

- [ ] Update `members` array
- [ ] Add new workspace dependencies
- [ ] Verify with `cargo metadata` (should not error)

### 1.6 Rename Crate Names

- [ ] `frontend/core/Cargo.toml`: `name = "client-frontend-core"`
- [ ] `frontend/cli/Cargo.toml`: `name = "client-frontend-cli"`
- [ ] `blockchain/sui/Cargo.toml`: `name = "client-blockchain-sui"`
- [ ] `blockchain/core/Cargo.toml`: `name = "client-blockchain-core"`

### 1.7 Update Import Paths

Update all `use` statements in moved crates:

- [ ] `client-core` → `client-frontend-core`
- [ ] `client-sui` → `client-blockchain-sui`

**Files to update:**
- `frontend/cli/Cargo.toml` (dependencies)
- `frontend/cli/src/**/*.rs` (imports)

### 1.8 Verify Build

```bash
cargo check --workspace
```

- [ ] Workspace builds without errors
- [ ] All path references updated correctly

**Checkpoint:** Commit Phase 1
```bash
git add -A
git commit -m "refactor(client): phase 1 - directory restructuring"
```

---

## Phase 2: Bootstrap Separation (2-3 hours)

### 2.1 Extract Frontend Config

Move frontend-specific config from `client-bootstrap` to `client-frontend-core`:

```bash
# Create new config in frontend-core
touch frontend/core/src/config.rs
```

- [ ] Copy `ClientConfig`, `MessageConfig`, `EffectVisibility` from `bootstrap/src/config.rs`
- [ ] Paste into `frontend/core/src/config.rs`
- [ ] Rename to `FrontendConfig`

### 2.2 Simplify Bootstrap Config

Edit `bootstrap/src/config.rs`:

- [ ] Remove `MessageConfig`
- [ ] Remove `EffectVisibility`
- [ ] Remove `ChannelConfig`
- [ ] Keep only runtime-related settings:
  - `enable_proving`
  - `enable_persistence`
  - `session_id`
  - `save_data_dir`
  - `checkpoint_interval`

### 2.3 Update RuntimeBuilder

Edit `bootstrap/src/builder.rs`:

```rust
pub struct RuntimeBuilder {
    oracle_factory: Arc<dyn OracleFactory>,
    enable_proving: bool,
    enable_persistence: bool,
    session_id: Option<String>,
    save_data_dir: Option<PathBuf>,
    checkpoint_interval: Option<u64>,
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            oracle_factory: Arc::new(ContentOracleFactory::default_paths()),
            enable_proving: false,
            enable_persistence: false,
            session_id: None,
            save_data_dir: None,
            checkpoint_interval: None,
        }
    }

    pub fn enable_proving(mut self, enable: bool) -> Self {
        self.enable_proving = enable;
        self
    }

    pub fn enable_persistence(mut self, enable: bool) -> Self {
        self.enable_persistence = enable;
        self
    }

    // ... builder methods
}
```

- [ ] Remove frontend config from `RuntimeBuilder`
- [ ] Add builder methods for runtime config
- [ ] Update `from_env()` to only read runtime env vars

### 2.4 Update Frontend CLI

Edit `frontend/cli/src/main.rs` (temporarily, will be removed later):

```rust
use client_frontend_core::FrontendConfig;
use client_bootstrap::RuntimeBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    let frontend_config = FrontendConfig::from_env();

    let runtime = RuntimeBuilder::new()
        .enable_proving(std::env::var("ENABLE_ZK_PROVING").is_ok())
        .enable_persistence(std::env::var("ENABLE_PERSISTENCE").is_ok())
        .build()
        .await?;

    CliApp::new(runtime, frontend_config).run().await
}
```

- [ ] Update CLI to use separate configs
- [ ] Verify CLI still works

### 2.5 Update Exports

- [ ] `frontend/core/src/lib.rs`: Export `FrontendConfig`
- [ ] `bootstrap/src/lib.rs`: Update exports

### 2.6 Verify Build

```bash
cargo check --workspace
cargo build -p client-frontend-cli --features stub
```

- [ ] Workspace builds successfully
- [ ] CLI builds successfully

**Checkpoint:** Commit Phase 2
```bash
git add -A
git commit -m "refactor(client): phase 2 - bootstrap separation"
```

---

## Phase 3: Blockchain Abstraction Layer (4-6 hours)

### 3.1 Create Blockchain Core Traits

Edit `blockchain/core/src/traits.rs`:

- [ ] Define `ProofSubmitter` trait
- [ ] Define `SessionManager` trait
- [ ] Define `BlockchainClient` trait
- [ ] Define `BlockchainError` enum
- [ ] Add comprehensive documentation

### 3.2 Create Common Types

Edit `blockchain/core/src/types.rs`:

- [ ] Define `SessionId` (newtype wrapper)
- [ ] Define `TransactionId` (newtype wrapper)
- [ ] Define `TransactionStatus` enum
- [ ] Define `SubmissionResult` struct
- [ ] Define `ProofMetadata` struct
- [ ] Define `SessionState` struct
- [ ] Define `BlockchainConfig` trait

### 3.3 Create Mock Implementation

Edit `blockchain/core/src/mock.rs`:

```rust
pub struct MockBlockchainClient {
    proofs: Arc<Mutex<Vec<ProofMetadata>>>,
    sessions: Arc<Mutex<HashMap<SessionId, SessionState>>>,
}

#[async_trait]
impl BlockchainClient for MockBlockchainClient {
    async fn submit_proof(...) -> Result<SubmissionResult> {
        // Mock implementation for testing
    }
    // ...
}
```

- [ ] Implement `MockBlockchainClient`
- [ ] Implement all traits for mock
- [ ] Add tests for mock implementation

### 3.4 Setup Blockchain Core Cargo.toml

Edit `blockchain/core/Cargo.toml`:

```toml
[package]
name = "client-blockchain-core"
version = "0.1.0"
edition = "2024"

[dependencies]
zk = { workspace = true }
game-core = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
async-trait = "0.1"
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

- [ ] Add all dependencies
- [ ] Verify builds: `cargo build -p client-blockchain-core`

### 3.5 Export Blockchain Core API

Edit `blockchain/core/src/lib.rs`:

```rust
pub mod traits;
pub mod types;

#[cfg(test)]
pub mod mock;

pub use traits::{BlockchainClient, ProofSubmitter, SessionManager, BlockchainError};
pub use types::{
    SessionId, TransactionId, TransactionStatus,
    SubmissionResult, ProofMetadata, SessionState,
    BlockchainConfig,
};

#[cfg(test)]
pub use mock::MockBlockchainClient;
```

- [ ] Export all public types
- [ ] Add crate-level documentation

### 3.6 Refactor Sui Implementation

Edit `blockchain/sui/src/client.rs` (new file):

```rust
use client_blockchain_core::*;

pub struct SuiBlockchainClient {
    sui_client: sui_sdk::SuiClient,
    config: SuiConfig,
    package_id: ObjectID,
}

#[async_trait]
impl BlockchainClient for SuiBlockchainClient {
    async fn submit_proof(
        &self,
        session_id: &SessionId,
        proof_data: ProofData,
    ) -> Result<SubmissionResult> {
        // 1. Convert proof (use existing converter.rs)
        let sui_proof = crate::converter::SuiProofConverter::convert(proof_data)?;

        // 2. Build transaction
        // 3. Submit to Sui
        // 4. Return result

        todo!("Implement Sui proof submission")
    }

    // Implement other trait methods...
}
```

- [ ] Create `client.rs` with `SuiBlockchainClient`
- [ ] Implement `BlockchainClient` trait
- [ ] Implement `ProofSubmitter` trait
- [ ] Implement `SessionManager` trait

### 3.7 Create SuiConfig

Edit `blockchain/sui/src/config.rs`:

```rust
use client_blockchain_core::BlockchainConfig;

pub struct SuiConfig {
    pub network: SuiNetwork,
    pub package_id: String,
    pub rpc_url: String,
    // ... other fields
}

impl SuiConfig {
    pub fn from_env() -> Result<Self> {
        // Load from environment variables
    }
}

impl BlockchainConfig for SuiConfig {
    fn network_name(&self) -> &str {
        match self.network {
            SuiNetwork::Mainnet => "sui-mainnet",
            SuiNetwork::Testnet => "sui-testnet",
            SuiNetwork::Local => "sui-local",
        }
    }

    fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    fn validate(&self) -> Result<(), String> {
        // Validate configuration
        Ok(())
    }
}
```

- [ ] Create `SuiConfig` struct
- [ ] Implement `BlockchainConfig` trait
- [ ] Add `from_env()` method
- [ ] Add validation logic

### 3.8 Update Sui Crate Exports

Edit `blockchain/sui/src/lib.rs`:

```rust
pub mod client;
pub mod config;
pub mod converter;
pub mod submitter;

pub use client::SuiBlockchainClient;
pub use config::SuiConfig;
pub use converter::SuiProofConverter;
```

- [ ] Update exports
- [ ] Add crate documentation

### 3.9 Update Sui Cargo.toml

Edit `blockchain/sui/Cargo.toml`:

```toml
[package]
name = "client-blockchain-sui"
version = "0.1.0"
edition = "2024"

[dependencies]
# Internal
client-blockchain-core = { workspace = true }
zk = { workspace = true }
game-core = { workspace = true }

# Sui SDK
sui-sdk = "0.68"
sp1-sui = { git = "https://github.com/SoundnessLabs/sp1-sui" }
sp1-sdk = { workspace = true }

# Utils
bincode = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
async-trait = "0.1"
tracing = { workspace = true }
```

- [ ] Add `client-blockchain-core` dependency
- [ ] Add `async-trait` dependency

### 3.10 Verify Blockchain Layer Builds

```bash
cargo build -p client-blockchain-core
cargo build -p client-blockchain-sui
cargo test -p client-blockchain-core
```

- [ ] Core builds successfully
- [ ] Sui implementation builds
- [ ] Mock tests pass

**Checkpoint:** Commit Phase 3
```bash
git add -A
git commit -m "refactor(client): phase 3 - blockchain abstraction layer"
```

---

## Phase 4: CLI Library Conversion (2-3 hours)

### 4.1 Remove Binary Definition

Edit `frontend/cli/Cargo.toml`:

- [ ] Remove `[[bin]]` section
- [ ] Remove `path = "src/main.rs"`

### 4.2 Create Library Root

Edit `frontend/cli/src/lib.rs`:

```rust
//! Terminal UI frontend for Dungeon game.

mod app;
mod config;
mod cursor;
mod event;
mod input;
mod presentation;
mod state;

pub use app::CliApp;
pub use config::CliConfig;

// Re-export for convenience
pub use client_frontend_core::FrontendConfig;
```

- [ ] Create `lib.rs` with public exports
- [ ] Export `CliApp` and `CliConfig`

### 4.3 Update CliApp for Blockchain Integration

Edit `frontend/cli/src/app.rs`:

```rust
use client_blockchain_core::BlockchainClient;

pub struct CliApp {
    runtime: Runtime,
    frontend_config: FrontendConfig,
    blockchain_client: Option<Box<dyn BlockchainClient>>,
    // ... existing fields
}

impl CliApp {
    pub fn builder(frontend_config: FrontendConfig) -> CliAppBuilder {
        CliAppBuilder {
            frontend_config,
            blockchain_client: None,
        }
    }

    pub fn attach_blockchain(&mut self, client: impl BlockchainClient + 'static) {
        self.blockchain_client = Some(Box::new(client));
    }
}

pub struct CliAppBuilder {
    frontend_config: FrontendConfig,
    blockchain_client: Option<Box<dyn BlockchainClient>>,
}

impl CliAppBuilder {
    pub fn blockchain(mut self, client: impl BlockchainClient + 'static) -> Self {
        self.blockchain_client = Some(Box::new(client));
        self
    }

    pub async fn build(self) -> Result<CliApp> {
        // Build runtime
        let runtime = RuntimeBuilder::new().build().await?;

        Ok(CliApp {
            runtime,
            frontend_config: self.frontend_config,
            blockchain_client: self.blockchain_client,
            // ... other fields
        })
    }
}
```

- [ ] Add `blockchain_client` field to `CliApp`
- [ ] Create `CliAppBuilder` for construction
- [ ] Add `attach_blockchain()` method

### 4.4 Extract Logging Setup

Create `frontend/cli/src/logging.rs`:

```rust
pub fn setup_logging(session_id: &Option<String>) -> Result<()> {
    // Move logging setup code from main.rs
}

fn get_log_directory() -> PathBuf {
    // Move platform-specific log directory logic
}
```

- [ ] Extract logging setup to separate module
- [ ] Make it reusable (will be used by main binary)

### 4.5 Update CliConfig

Edit `frontend/cli/src/config.rs`:

- [ ] Keep CLI-specific config (keybindings, etc.)
- [ ] Import `FrontendConfig` from `client-frontend-core`

### 4.6 Delete main.rs

```bash
rm frontend/cli/src/main.rs
```

- [ ] Delete `frontend/cli/src/main.rs`

### 4.7 Verify CLI Library Builds

```bash
cargo build -p client-frontend-cli --features stub
```

- [ ] CLI builds as library
- [ ] No binary target

**Checkpoint:** Commit Phase 4
```bash
git add -A
git commit -m "refactor(client): phase 4 - CLI library conversion"
```

---

## Phase 5: Composable Binary (3-4 hours)

### 5.1 Create Main Binary Cargo.toml

Edit `crates/client/Cargo.toml`:

```toml
[package]
name = "dungeon-client"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "dungeon"
path = "src/main.rs"

[features]
default = ["frontend-cli", "zkvm-stub"]

# Frontend selection
frontend-cli = ["dep:client-frontend-cli"]
# frontend-gui = []  # Future

# Blockchain selection (optional)
blockchain-sui = ["dep:client-blockchain-sui", "dep:client-blockchain-core"]
# blockchain-ethereum = []  # Future

# ZK backend (propagate to bootstrap)
zkvm-risc0 = ["client-bootstrap/risc0"]
zkvm-sp1 = ["client-bootstrap/sp1"]
zkvm-stub = ["client-bootstrap/stub"]
zkvm-arkworks = ["client-bootstrap/arkworks"]

[dependencies]
# Always included
client-bootstrap = { workspace = true }
anyhow = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
tracing-appender = { workspace = true }
dotenvy = { workspace = true }

# Feature-gated frontends
client-frontend-cli = { workspace = true, optional = true }

# Feature-gated blockchains
client-blockchain-core = { workspace = true, optional = true }
client-blockchain-sui = { workspace = true, optional = true }
```

- [ ] Create complete `Cargo.toml` with features
- [ ] Add all dependencies with correct feature gates

### 5.2 Implement Main Entry Point

Edit `crates/client/src/main.rs`:

```rust
use anyhow::Result;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists
    let _ = dotenvy::dotenv();

    // Setup logging
    setup_logging()?;

    // Run selected frontend
    #[cfg(feature = "frontend-cli")]
    {
        run_cli().await?;
    }

    #[cfg(feature = "frontend-gui")]
    {
        run_gui().await?;
    }

    #[cfg(not(any(feature = "frontend-cli", feature = "frontend-gui")))]
    {
        compile_error!("At least one frontend feature must be enabled");
    }

    Ok(())
}

#[cfg(feature = "frontend-cli")]
async fn run_cli() -> Result<()> {
    use client_frontend_cli::{CliApp, CliConfig, FrontendConfig};

    let frontend_config = FrontendConfig::from_env();
    let cli_config = CliConfig::from_env();

    let mut builder = CliApp::builder(frontend_config, cli_config);

    // Attach blockchain client if enabled
    #[cfg(feature = "blockchain-sui")]
    {
        let sui_client = initialize_sui_client().await?;
        builder = builder.blockchain(sui_client);
    }

    let app = builder.build().await?;
    app.run().await
}

#[cfg(feature = "blockchain-sui")]
async fn initialize_sui_client() -> Result<impl client_blockchain_core::BlockchainClient> {
    use client_blockchain_sui::{SuiBlockchainClient, SuiConfig};

    let config = SuiConfig::from_env()?;
    SuiBlockchainClient::new(config).await
}

fn setup_logging() -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Get session ID from environment
    let session_id = std::env::var("GAME_SESSION_ID").ok().unwrap_or_else(|| {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("session_{}", timestamp)
    });

    // Determine log directory
    let log_dir = get_log_directory();
    let session_log_dir = log_dir.join(&session_id);
    std::fs::create_dir_all(&session_log_dir)?;

    // Setup file appender
    let file_appender = tracing_appender::rolling::never(&session_log_dir, "client.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Create env filter
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Setup file layer
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(true);

    // Initialize subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .init();

    std::mem::forget(_guard);

    tracing::info!("Logging initialized: session={}", session_id);
    tracing::info!("Log file: {}/client.log", session_log_dir.display());

    Ok(())
}

fn get_log_directory() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut path = std::path::PathBuf::from(home);
            path.push("Library/Caches/dungeon/logs");
            return path;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(xdg_cache) = std::env::var_os("XDG_CACHE_HOME") {
            let mut path = std::path::PathBuf::from(xdg_cache);
            path.push("dungeon/logs");
            return path;
        } else if let Some(home) = std::env::var_os("HOME") {
            let mut path = std::path::PathBuf::from(home);
            path.push(".cache/dungeon/logs");
            return path;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
            let mut path = std::path::PathBuf::from(local_appdata);
            path.push("dungeon/logs");
            return path;
        }
    }

    std::path::PathBuf::from("/tmp/dungeon/logs")
}
```

- [ ] Implement complete `main.rs` with feature gates
- [ ] Add logging setup
- [ ] Add blockchain initialization (feature-gated)

### 5.3 Verify Binary Builds

Test different feature combinations:

```bash
# CLI only (default)
cargo build -p dungeon-client

# CLI + Sui
cargo build -p dungeon-client --features "frontend-cli,blockchain-sui,zkvm-sp1"

# Different ZK backends
cargo build -p dungeon-client --features "frontend-cli,zkvm-risc0"
cargo build -p dungeon-client --features "frontend-cli,zkvm-sp1"
cargo build -p dungeon-client --features "frontend-cli,zkvm-stub"
```

- [ ] Default build works
- [ ] CLI + Sui build works
- [ ] All zkvm backends build

### 5.4 Update Justfile

Edit `justfile`:

```bash
# New binary-based commands
run-client backend="stub":
    cargo run -p dungeon-client \
        --no-default-features \
        --features "frontend-cli,zkvm-{{backend}}"

run-client-sui backend="sp1":
    cargo run -p dungeon-client \
        --no-default-features \
        --features "frontend-cli,blockchain-sui,zkvm-{{backend}}"

# Alias 'run' to new command
run backend="stub":
    just run-client {{backend}}

# Build all combinations for testing
build-all-combos:
    @echo "Building all feature combinations..."
    cargo build -p dungeon-client --no-default-features --features "frontend-cli,zkvm-stub"
    cargo build -p dungeon-client --no-default-features --features "frontend-cli,zkvm-sp1"
    cargo build -p dungeon-client --no-default-features --features "frontend-cli,zkvm-risc0"
    cargo build -p dungeon-client --no-default-features --features "frontend-cli,blockchain-sui,zkvm-stub"
    cargo build -p dungeon-client --no-default-features --features "frontend-cli,blockchain-sui,zkvm-sp1"
    @echo "All combinations built successfully!"
```

- [ ] Add new `run-client` command
- [ ] Add `run-client-sui` command
- [ ] Update default `run` command
- [ ] Add `build-all-combos` for testing

### 5.5 Test Binary Execution

```bash
# Test default run
just run stub

# Test with Sui (if ready)
just run-client-sui sp1
```

- [ ] Binary runs successfully
- [ ] CLI interface works
- [ ] Logging works

**Checkpoint:** Commit Phase 5
```bash
git add -A
git commit -m "refactor(client): phase 5 - composable binary"
```

---

## Phase 6: Testing & Documentation (2-3 hours)

### 6.1 Test All Feature Combinations

```bash
# Test matrix
just build-all-combos
```

- [ ] `frontend-cli,zkvm-stub` builds and runs
- [ ] `frontend-cli,zkvm-sp1` builds and runs
- [ ] `frontend-cli,zkvm-risc0` builds and runs
- [ ] `frontend-cli,blockchain-sui,zkvm-stub` builds
- [ ] `frontend-cli,blockchain-sui,zkvm-sp1` builds

### 6.2 Write Integration Tests

Create `crates/client/tests/integration_test.rs`:

```rust
#[cfg(feature = "blockchain-sui")]
#[tokio::test]
async fn test_sui_blockchain_integration() {
    use client_blockchain_sui::{SuiBlockchainClient, SuiConfig};
    use client_blockchain_core::BlockchainClient;

    // Test with mock or testnet
    // ...
}
```

- [ ] Write integration test for blockchain layer
- [ ] Test mock blockchain client
- [ ] Run tests: `cargo test -p dungeon-client`

### 6.3 Update CLAUDE.md

Edit `/home/wonjae/code/dungeon/CLAUDE.md`:

Update the following sections:

**Client Architecture:**
```markdown
### Client Architecture

The client layer is organized into composable components:

```
crates/client/              # Binary crate (composable)
├── bootstrap/              # Common runtime initialization
├── frontend/               # User interface implementations
│   ├── core/              # UI primitives and configuration
│   └── cli/               # Terminal UI (library)
└── blockchain/            # Blockchain integrations
    ├── core/              # Blockchain abstraction (traits)
    └── sui/               # Sui implementation
```

**Feature Flags:**
- `frontend-cli` - Terminal UI (default)
- `blockchain-sui` - Sui blockchain integration (optional)
- `zkvm-{risc0,sp1,stub,arkworks}` - ZK backend selection

**Build Examples:**
```bash
# CLI only (local play)
cargo run -p dungeon-client --features "frontend-cli,zkvm-stub"

# CLI + Sui blockchain
cargo run -p dungeon-client --features "frontend-cli,blockchain-sui,zkvm-sp1"
```
```

- [ ] Update client architecture section
- [ ] Add feature flags documentation
- [ ] Add build examples
- [ ] Update crate structure diagram

### 6.4 Update README (if exists)

- [ ] Update build instructions
- [ ] Add feature flag documentation
- [ ] Update quickstart guide

### 6.5 Create Migration Guide

Create `docs/migration-guide-client-restructuring.md`:

```markdown
# Migration Guide: Client Restructuring

For developers who have existing code using the old client structure.

## Import Path Changes

| Old | New |
|-----|-----|
| `client-core` | `client-frontend-core` |
| `client-sui` | `client-blockchain-sui` |
| `ClientConfig` | `FrontendConfig` (in `client-frontend-core`) |

## Build Command Changes

| Old | New |
|-----|-----|
| `cargo run -p client-cli` | `cargo run -p dungeon-client` |
| `ENABLE_PROVING=1 cargo run -p client-cli` | `cargo run -p dungeon-client --features "frontend-cli,zkvm-sp1"` |

## Code Changes

### Before
```rust
use client_core::EventConsumer;
use client_bootstrap::ClientConfig;

let config = ClientConfig::from_env();
```

### After
```rust
use client_frontend_core::{EventConsumer, FrontendConfig};

let config = FrontendConfig::from_env();
```
```

- [ ] Create migration guide
- [ ] Document all breaking changes
- [ ] Provide before/after examples

### 6.6 Verify Documentation

- [ ] All markdown files render correctly
- [ ] Code examples compile
- [ ] Links are not broken

**Final Checkpoint:** Commit Phase 6
```bash
git add -A
git commit -m "refactor(client): phase 6 - testing and documentation"
```

---

## Post-Migration Verification

### Smoke Tests

```bash
# 1. Clean build
cargo clean
cargo build --workspace

# 2. Run tests
cargo test --workspace --features stub

# 3. Run binary with different features
just run stub
just run sp1
just run-client-sui sp1  # If Sui implementation ready

# 4. Check formatting
cargo fmt --all --check

# 5. Check lints
cargo clippy --workspace --all-targets --features stub
```

- [ ] Clean build succeeds
- [ ] All workspace tests pass
- [ ] Binary runs with all feature combinations
- [ ] Formatting is correct
- [ ] No clippy warnings

### Final Review

- [ ] All phases completed
- [ ] All checkpoints committed
- [ ] Documentation updated
- [ ] Tests passing
- [ ] No regressions in functionality

---

## Rollback Plan (If Needed)

If something goes wrong:

```bash
# 1. Stash current work
git stash

# 2. Return to backup branch
git checkout backup/pre-restructuring

# 3. Review what went wrong
git diff refactor/client-restructuring

# 4. Fix issues and retry specific phase
```

---

## Completion

When all checkboxes are marked:

1. Squash commits (optional):
   ```bash
   git rebase -i main
   ```

2. Create pull request:
   ```bash
   git push origin refactor/client-restructuring
   ```

3. Request review

4. Merge to main after approval

**Status:** Not Started
**Started:** YYYY-MM-DD
**Completed:** YYYY-MM-DD
**Total Time:** X hours
