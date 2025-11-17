# client-sui: Sui Blockchain Integration

Handles proof submission to Sui blockchain for the Dungeon game.

## Overview

This crate provides the integration layer between the game's proof generation (`zk` crate) and Sui blockchain. It handles:

- **Proof Format Conversion**: SP1 gnark → Sui arkworks
- **Transaction Construction**: Building Sui Move contract calls
- **On-Chain Submission**: Signing and executing transactions

## Architecture Philosophy

**Conversion at Client Layer**: The `zk` crate remains blockchain-agnostic. All blockchain-specific logic (including proof format conversion) lives here in `client-sui`.

**Why?**
- `zk` crate: Pure proof generation, no blockchain dependencies
- `client-sui`: Blockchain integration, conversion, submission
- Proof storage: Consistent format across all backends (RISC0, SP1)

## Usage

### Basic Workflow

```rust
use client_sui::{SuiProofConverter, SuiProofSubmitter};
use zk::ProofData;

// 1. Load proof from storage (original SP1 format)
let proof_data: ProofData = load_from_disk()?;

// 2. Convert to Sui format
let sui_proof = SuiProofConverter::convert(proof_data)?;

// 3. Submit to Sui
let submitter = SuiProofSubmitter::new(sui_client).await?;
let tx_digest = submitter.submit_proof(vk_object_id, proof_data).await?;
```

### Verifying Key Deployment

```rust
// One-time setup: Deploy VK to Sui
let vk_bytes = sui_proof.verifying_key;
let vk_object_id = submitter
    .deploy_verifying_key(vk_bytes, 1)
    .await?;
```

## Modules

- `converter`: SP1 → Sui proof format conversion
- `submitter`: Sui transaction construction and submission

## Dependencies

- `sp1-sui`: gnark → arkworks conversion utility
- `sui-sdk`: Sui blockchain client
- `zk`: Game proof data structures

## See Also

- [SP1-Sui Integration Guide](../../../docs/SP1_SUI_INTEGRATION.md)
- [Move Contract: proof_verifier.move](../../../contracts/move/sources/proof_verifier.move)
