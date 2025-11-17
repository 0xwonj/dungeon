# Client Crate Restructuring Plan

## Executive Summary

This document outlines the plan to restructure the `client` crate architecture to:
1. Separate frontend (UI) and blockchain (infrastructure) concerns
2. Create a composable binary system using feature flags
3. Enable flexible combinations of frontends and blockchain backends
4. Follow the same abstraction pattern as the `zk` crate (trait-based backends)

## Current Structure (As-Is)

```
crates/client/
├── bootstrap/      # client-bootstrap: Runtime setup + frontend config (mixed)
├── core/           # client-core: UI primitives
├── cli/            # client-cli: Terminal app (binary)
└── sui/            # client-sui: Sui blockchain integration
```

**Problems:**
- ❌ Bootstrap mixes runtime initialization with frontend-specific config
- ❌ No abstraction layer for blockchain clients (Sui is directly implemented)
- ❌ CLI is a separate binary (hard to compose with blockchain features)
- ❌ Adding new frontends (GUI, web) or blockchains (Ethereum) requires significant refactoring

## Target Structure (To-Be)

```
crates/client/              # ⭐ Binary crate (composable via features)
├── Cargo.toml              # Feature flags: frontend-*, blockchain-*, zkvm-*
├── src/
│   └── main.rs             # Feature-gated entry point
│
├── bootstrap/              # Common runtime initialization
│   └── src/
│       ├── runtime.rs      # RuntimeBuilder, RuntimeSetup
│       └── oracles.rs      # OracleFactory, ContentOracleFactory
│
├── frontend/
│   ├── core/               # client-frontend-core
│   │   └── src/
│   │       ├── config.rs   # FrontendConfig (UI-specific)
│   │       ├── event.rs
│   │       ├── message.rs
│   │       ├── view_model/
│   │       └── services/
│   │
│   └── cli/                # client-frontend-cli (library, no binary)
│       └── src/
│           ├── lib.rs      # Export CliApp, CliConfig
│           ├── app.rs
│           ├── cursor/
│           └── presentation/
│
└── blockchain/
    ├── core/               # client-blockchain-core
    │   └── src/
    │       ├── traits.rs   # BlockchainClient, ProofSubmitter, SessionManager
    │       ├── types.rs    # SessionId, TransactionId, ProofMetadata
    │       └── mock.rs     # MockBlockchainClient (testing)
    │
    └── sui/                # client-blockchain-sui
        └── src/
            ├── lib.rs
            ├── client.rs   # SuiBlockchainClient (trait implementation)
            ├── converter.rs # SP1 → Sui conversion
            ├── submitter.rs # ProofSubmitter implementation
            └── config.rs   # SuiConfig
```

## Design Principles

### 1. Trait-Based Abstraction (Following `zk` Crate Pattern)

Just like `zk::Prover` provides a common interface for RISC0/SP1/Stub:

```rust
// zk crate
pub trait Prover {
    fn prove(&self, ...) -> Result<ProofData>;
    fn verify(&self, proof: &ProofData) -> Result<bool>;
}

// Implementations: Risc0Prover, Sp1Prover, StubProver
```

We apply the same pattern to blockchain clients:

```rust
// client-blockchain-core
pub trait BlockchainClient {
    async fn submit_proof(&self, ...) -> Result<SubmissionResult>;
    async fn create_session(&self, ...) -> Result<SessionId>;
    async fn get_session_state(&self, ...) -> Result<SessionState>;
}

// Implementations: SuiBlockchainClient, EthereumBlockchainClient (future)
```

### 2. Separation of Concerns

| Crate | Responsibility | Used By |
|-------|---------------|---------|
| `client-bootstrap` | Runtime initialization (oracle, AI, persistence) | All clients |
| `client-frontend-core` | UI primitives (events, messages, view models, config) | Frontend implementations |
| `client-frontend-cli` | Terminal UI implementation | Main binary (feature-gated) |
| `client-blockchain-core` | Blockchain abstraction (traits, types) | Blockchain implementations |
| `client-blockchain-sui` | Sui-specific implementation | Main binary (feature-gated) |
| `client` (binary) | Composable entry point | End users |

### 3. Feature Flag Composition

Users can build different configurations:

```bash
# CLI only, no blockchain (local play)
cargo build --features "frontend-cli,zkvm-stub"

# CLI + Sui blockchain (on-chain proofs)
cargo build --features "frontend-cli,blockchain-sui,zkvm-sp1"

# Future: GUI + Ethereum
cargo build --features "frontend-gui,blockchain-ethereum,zkvm-risc0"
```

## Detailed Design

### Bootstrap Separation

**Before:** Bootstrap mixed runtime init + frontend config

```rust
// client-bootstrap (before)
pub struct ClientConfig {
    pub enable_proving: bool,        // Runtime
    pub enable_persistence: bool,    // Runtime
    pub messages: MessageConfig,     // Frontend! ❌
    pub channels: ChannelConfig,     // Frontend! ❌
}
```

**After:** Clean separation

```rust
// client-bootstrap (after) - Only runtime concerns
pub struct RuntimeBuilder {
    oracle_factory: Arc<dyn OracleFactory>,
    enable_proving: bool,
    enable_persistence: bool,
    session_id: Option<String>,
    // No frontend config
}

// client-frontend-core - Only UI concerns
pub struct FrontendConfig {
    pub channels: ChannelConfig,
    pub messages: MessageConfig,
    pub effect_visibility: EffectVisibility,
}
```

### Blockchain Abstraction Layer

#### Core Traits (`client-blockchain-core`)

```rust
// traits.rs
#[async_trait]
pub trait ProofSubmitter: Send + Sync {
    async fn submit_proof(
        &self,
        session_id: &SessionId,
        proof_data: ProofData,
    ) -> Result<SubmissionResult>;

    async fn submit_batch(
        &self,
        session_id: &SessionId,
        proofs: Vec<ProofData>,
    ) -> Result<Vec<SubmissionResult>>;

    async fn estimate_gas(
        &self,
        session_id: &SessionId,
        proof_data: &ProofData,
    ) -> Result<u64>;
}

#[async_trait]
pub trait SessionManager: Send + Sync {
    async fn create_session(&self, oracle_root: [u8; 32]) -> Result<SessionId>;
    async fn get_session_state(&self, session_id: &SessionId) -> Result<SessionState>;
    async fn finalize_session(&self, session_id: &SessionId) -> Result<TransactionId>;
}

#[async_trait]
pub trait BlockchainClient: ProofSubmitter + SessionManager + Send + Sync {
    async fn list_pending_proofs(&self) -> Result<Vec<ProofMetadata>>;
    async fn submit_all_pending(&self, session_id: &SessionId) -> Result<Vec<SubmissionResult>>;
    async fn health_check(&self) -> Result<()>;
    fn config(&self) -> &dyn BlockchainConfig;
}
```

#### Common Types (`client-blockchain-core`)

```rust
// types.rs
pub struct SessionId(pub Vec<u8>);  // Blockchain-agnostic
pub struct TransactionId(pub Vec<u8>);

pub enum TransactionStatus {
    Pending,
    Confirmed { block_height: u64 },
    Failed { error: String },
}

pub struct SubmissionResult {
    pub transaction_id: TransactionId,
    pub gas_cost: u64,
    pub status: TransactionStatus,
}

pub struct ProofMetadata {
    pub nonce: u64,
    pub proof_data: ProofData,
    pub estimated_gas: Option<u64>,
    pub submitted: bool,
    pub transaction_id: Option<TransactionId>,
}
```

#### Sui Implementation (`client-blockchain-sui`)

```rust
// client.rs
pub struct SuiBlockchainClient {
    sui_client: sui_sdk::SuiClient,
    config: SuiConfig,
}

#[async_trait]
impl BlockchainClient for SuiBlockchainClient {
    async fn submit_proof(&self, session_id: &SessionId, proof_data: ProofData) -> Result<SubmissionResult> {
        // 1. Convert ProofData to Sui format (using existing converter.rs)
        let sui_proof = SuiProofConverter::convert(proof_data)?;

        // 2. Build transaction
        let tx = self.build_verification_tx(session_id, sui_proof)?;

        // 3. Sign and submit
        let response = self.sui_client.sign_and_execute_transaction(tx).await?;

        // 4. Return result
        Ok(SubmissionResult { ... })
    }

    // ... implement other trait methods
}
```

### Composable Binary (`crates/client`)

#### Feature Flags (`Cargo.toml`)

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

# Frontend selection (mutually exclusive, choose one)
frontend-cli = ["dep:client-frontend-cli"]
frontend-gui = []  # Future: Graphical UI
frontend-web = []  # Future: Web UI

# Blockchain selection (optional, multiple allowed)
blockchain-sui = ["dep:client-blockchain-sui"]
blockchain-ethereum = []  # Future
blockchain-starknet = []  # Future

# ZK backend (mutually exclusive, propagate to bootstrap)
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
dotenvy = { workspace = true }

# Feature-gated frontends
client-frontend-cli = { workspace = true, optional = true }

# Feature-gated blockchains
client-blockchain-sui = { workspace = true, optional = true }
client-blockchain-core = { workspace = true, optional = true }
```

#### Entry Point (`src/main.rs`)

```rust
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    let _ = dotenvy::dotenv();

    // Setup logging
    setup_logging()?;

    // Compile-time frontend selection
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
    use client_frontend_cli::CliApp;

    let config = client_frontend_cli::CliConfig::from_env();

    let mut app = CliApp::builder(config)
        .build()
        .await?;

    // Optionally attach blockchain client
    #[cfg(feature = "blockchain-sui")]
    {
        let sui_client = initialize_sui_client().await?;
        app.attach_blockchain(sui_client);
    }

    app.run().await
}

#[cfg(feature = "blockchain-sui")]
async fn initialize_sui_client() -> Result<impl client_blockchain_core::BlockchainClient> {
    use client_blockchain_sui::SuiBlockchainClient;

    let config = client_blockchain_sui::SuiConfig::from_env()?;
    SuiBlockchainClient::new(config).await
}

fn setup_logging() -> Result<()> {
    // Common logging setup for all frontends
    // (Extracted from current cli/src/main.rs)
    // ...
}
```

## Migration Plan

### Phase 1: Directory Restructuring

**Estimated Time:** 1-2 hours

```bash
cd crates/client

# 1. Create new directory structure
mkdir -p frontend blockchain src

# 2. Move existing crates
mv core frontend/
mv cli frontend/
mv sui blockchain/

# 3. Create main binary structure
touch Cargo.toml src/main.rs
```

**Deliverables:**
- New directory structure in place
- No code changes yet (just file moves)

### Phase 2: Bootstrap Separation

**Estimated Time:** 2-3 hours

**Tasks:**
1. Extract frontend config from `client-bootstrap`
   - Move `ClientConfig`, `MessageConfig`, `EffectVisibility` to `client-frontend-core/src/config.rs`
   - Keep only runtime-related config in `client-bootstrap`

2. Update `RuntimeBuilder` to remove frontend dependencies
   ```rust
   // Before
   pub fn build(self) -> Result<RuntimeSetup> {
       let config = self.client_config; // Has frontend stuff
       // ...
   }

   // After
   pub fn build(self) -> Result<RuntimeSetup> {
       // Only runtime config
       // ...
   }
   ```

3. Update imports across crates

**Deliverables:**
- `client-bootstrap` only handles runtime initialization
- `client-frontend-core` owns all frontend configuration

### Phase 3: Blockchain Abstraction Layer

**Estimated Time:** 4-6 hours

**Tasks:**
1. Create `client-blockchain-core` crate
   - Define `BlockchainClient`, `ProofSubmitter`, `SessionManager` traits
   - Define common types: `SessionId`, `TransactionId`, `SubmissionResult`, etc.
   - Implement `MockBlockchainClient` for testing

2. Refactor `client-sui` → `client-blockchain-sui`
   - Implement `BlockchainClient` trait for `SuiBlockchainClient`
   - Keep existing `converter.rs` (SP1 → Sui conversion)
   - Adapt `submitter.rs` to use trait methods
   - Create `SuiConfig` implementing `BlockchainConfig` trait

3. Add `SuiConfig::from_env()` for environment variable loading
   ```rust
   pub struct SuiConfig {
       pub network: SuiNetwork,
       pub package_id: ObjectID,
       pub signer_keypair: SuiKeyPair,
       pub gas_budget: u64,
   }

   impl SuiConfig {
       pub fn from_env() -> Result<Self> {
           // Load from environment variables
       }
   }
   ```

**Deliverables:**
- `client-blockchain-core` with trait definitions
- `client-blockchain-sui` implementing traits
- Mock implementation for testing

### Phase 4: CLI Library Conversion

**Estimated Time:** 2-3 hours

**Tasks:**
1. Remove `[[bin]]` section from `client-frontend-cli/Cargo.toml`
2. Delete `client-frontend-cli/src/main.rs`
3. Export public API from `client-frontend-cli/src/lib.rs`
   ```rust
   pub mod app;
   pub mod config;
   // ... other modules

   pub use app::CliApp;
   pub use config::CliConfig;
   ```

4. Update `CliApp` to support optional blockchain client
   ```rust
   pub struct CliApp {
       runtime: Runtime,
       blockchain_client: Option<Box<dyn BlockchainClient>>,
       // ...
   }

   impl CliApp {
       pub fn attach_blockchain(&mut self, client: impl BlockchainClient + 'static) {
           self.blockchain_client = Some(Box::new(client));
       }
   }
   ```

**Deliverables:**
- `client-frontend-cli` is a library crate
- `CliApp` can optionally integrate blockchain client

### Phase 5: Composable Binary

**Estimated Time:** 3-4 hours

**Tasks:**
1. Create `crates/client/Cargo.toml` with feature flags
2. Implement `crates/client/src/main.rs` with feature-gated entry points
3. Extract logging setup to shared function
4. Update workspace `Cargo.toml` to include new crates
5. Update `justfile` with new build commands

**Deliverables:**
- Single `dungeon` binary with feature flag composition
- Updated build system and justfile

### Phase 6: Testing & Documentation

**Estimated Time:** 2-3 hours

**Tasks:**
1. Test all feature combinations:
   - `frontend-cli` only
   - `frontend-cli + blockchain-sui`
   - Different zkvm backends (risc0, sp1, stub)

2. Update CLAUDE.md with new architecture
3. Write integration tests for blockchain abstraction layer
4. Update README with new build instructions

**Deliverables:**
- All feature combinations working
- Updated documentation
- Integration tests

## Total Estimated Time

**16-21 hours** across 6 phases

## Build Examples (After Migration)

### Development (Local Play, No Blockchain)

```bash
# Fast iteration with stub prover
cargo run -p dungeon-client --features "frontend-cli,zkvm-stub"

# Default (same as above)
cargo run -p dungeon-client
```

### Production (On-Chain Proofs)

```bash
# CLI + Sui + SP1 (recommended for cross-platform)
cargo run -p dungeon-client --features "frontend-cli,blockchain-sui,zkvm-sp1"

# CLI + Sui + RISC0 (Linux only, Groth16)
cargo run -p dungeon-client --features "frontend-cli,blockchain-sui,zkvm-risc0"
```

### Just Commands (Updated)

```bash
# justfile additions
run-sui backend="sp1":
    cargo run -p dungeon-client \
        --no-default-features \
        --features "frontend-cli,blockchain-sui,zkvm-{{backend}}"

run backend="stub":
    cargo run -p dungeon-client \
        --no-default-features \
        --features "frontend-cli,zkvm-{{backend}}"

build-all-combinations:
    just build-combo frontend-cli zkvm-stub
    just build-combo frontend-cli,blockchain-sui zkvm-sp1
    just build-combo frontend-cli,blockchain-sui zkvm-risc0

build-combo features:
    cargo build -p dungeon-client --no-default-features --features "{{features}}"
```

## Dependency Graph (After Migration)

```
dungeon-client (binary)
    ├── client-bootstrap
    │       ├── runtime
    │       ├── game-content
    │       └── game-core
    │
    ├── client-frontend-cli (optional)
    │       ├── client-frontend-core
    │       │       ├── runtime
    │       │       └── game-core
    │       └── client-bootstrap
    │
    └── client-blockchain-sui (optional)
            ├── client-blockchain-core
            │       └── zk
            ├── sui-sdk
            ├── sp1-sui
            └── zk
```

## Benefits Summary

### ✅ Flexibility
- Users can choose frontend (CLI, GUI, Web)
- Users can enable/disable blockchain integration
- Different blockchain backends (Sui, Ethereum, StarkNet)

### ✅ Binary Size Optimization
- Blockchain SDK only compiled when needed
- Feature flags eliminate dead code

### ✅ Maintainability
- Clear separation of concerns (frontend vs blockchain)
- Trait-based abstraction (easy to add new backends)
- Single binary entry point (reduced user confusion)

### ✅ Extensibility
- New frontends: just implement `FrontendApp` interface
- New blockchains: just implement `BlockchainClient` trait
- Composable via feature flags

### ✅ Testability
- Mock blockchain client for testing without network
- Frontend and blockchain can be tested independently

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking changes to existing CLI users | High | Keep backward compatibility in config loading |
| Feature flag explosion (too many combinations) | Medium | Document recommended combinations, test matrix |
| Blockchain abstraction too generic | Medium | Start with Sui, refactor when adding 2nd blockchain |
| Binary size increase with all features | Low | Users build with specific features needed |

## Success Criteria

- ✅ Single `dungeon` binary works with different feature combinations
- ✅ CLI works both with and without blockchain integration
- ✅ Blockchain client abstraction supports Sui (extensible to others)
- ✅ All existing functionality preserved
- ✅ Build time not significantly increased
- ✅ Documentation updated and accurate

## Next Steps

1. Review this plan with team
2. Get approval for breaking changes (if any)
3. Create tracking issue/epic in GitHub
4. Start Phase 1 (directory restructuring)
5. Iterate through phases with testing at each step

---

**Document Version:** 1.0
**Last Updated:** 2025-11-17
**Author:** Claude Code
**Status:** DRAFT - Pending Review
