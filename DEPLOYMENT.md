# Sui Blockchain Deployment Guide

This guide covers the complete workflow for deploying Dungeon's Move contracts to Sui blockchain networks (local, testnet, or mainnet).

## Prerequisites

- Sui CLI installed (via suiup)
- Sui localnet running (for local deployment)
- Sufficient SUI tokens for gas (from faucet for testnet/local, purchased for mainnet)

## Quick Start (Local Network)

```bash
# 1. Start local network with faucet
just sui-localnet

# 2. In a new terminal, generate address and fund it
cargo xtask sui keygen --alias local-dev
sui client faucet

# 3. Deploy contracts
cd contracts/move
sui move build
sui client publish --gas-budget 10000000000 --with-unpublished-dependencies --json

# 4. Save package ID and setup VK
# Copy the package ID from the output, then:
cargo xtask sui setup --network local
```

## Detailed Workflow

### 1. Network Setup

#### Local Network
```bash
# Start Sui local network with faucet (foreground)
just sui-localnet

# Or manually:
sui start --force-regenesis --with-faucet

# Network will be available at:
# - RPC: http://127.0.0.1:9000
# - Faucet: http://0.0.0.0:9123
```

#### Testnet
```bash
# Switch to testnet
sui client switch --env testnet

# Verify connection
sui client envs
```

#### Mainnet
```bash
# Switch to mainnet
sui client switch --env mainnet

# Verify connection
sui client envs
```

### 2. Address Management

#### Generate New Address

Using xtask (with alias support):
```bash
# Generate with alias
cargo xtask sui keygen --alias my-deployer

# Generate with specific scheme
cargo xtask sui keygen --alias my-deployer --scheme ed25519
```

Using Sui CLI directly:
```bash
# Generate ed25519 address (default)
sui client new-address ed25519

# Generate secp256k1 address
sui client new-address secp256k1

# Generate secp256r1 address
sui client new-address secp256r1
```

#### Set Active Address

By alias (xtask keygen only):
```bash
export SUI_ACTIVE_ALIAS=my-deployer
```

By address (Sui CLI):
```bash
sui client switch --address 0x...
```

#### Check Address Status
```bash
# View all addresses
sui client addresses

# Check gas coins
sui client gas

# Check all objects
sui client objects
```

### 3. Fund Address

#### Local Network
```bash
# Request tokens from local faucet
sui client faucet

# Verify balance
sui client gas
```

#### Testnet
```bash
# Request tokens from testnet faucet
sui client faucet

# Or use the web faucet:
# https://faucet.testnet.sui.io/
```

#### Mainnet
```bash
# Purchase SUI from an exchange
# Transfer to your address
```

### 4. Deploy Contracts

```bash
# Navigate to contracts directory
cd contracts/move

# Build the Move package
sui move build

# Publish to blockchain
# Gas budget: 10 SUI = 10,000,000,000 MIST
sui client publish \
  --gas-budget 10000000000 \
  --with-unpublished-dependencies \
  --json

# Example output:
# {
#   "objectChanges": [
#     {
#       "type": "published",
#       "packageId": "0x1234...",
#       ...
#     }
#   ],
#   ...
# }
```

**Important Flags:**
- `--gas-budget`: Maximum gas to spend (in MIST, 1 SUI = 1,000,000,000 MIST)
- `--with-unpublished-dependencies`: Include unpublished dependencies (like Walrus) in the package
- `--json`: Output in JSON format for parsing

**Save Package ID:**
Manually create `deployment/{network}.toml`:
```toml
network = "local"  # or "testnet", "mainnet"
package_id = "0x1234..."  # Copy from publish output
deployed_at = "2025-01-18T10:30:00Z"
```

### 5. Register Verifying Key

After deployment, register the SP1 Groth16 verifying key on-chain:

```bash
# Register VK for the deployed package
cargo xtask sui setup --network local

# Or for testnet:
cargo xtask sui setup --network testnet

# Skip VK registration if already done:
cargo xtask sui setup --network local --skip-vk
```

This command will:
1. Load deployment info from `deployment/{network}.toml`
2. Connect to the Sui network
3. Call `proof_verifier::create_verifying_key` with SP1 VK bytes
4. Save the VK object ID back to `deployment/{network}.toml`

**Output:**
```
🔧 Setting up Sui deployment for local...

📝 Registering verifying key...
   Using SP1 Groth16 VK v5.0.0
   VK size: 128 bytes
   Connecting to: http://127.0.0.1:9000
   Using address: 0x...
   Submitting transaction...
   Transaction digest: ...

✅ VK registered successfully!
   VK Object ID: 0xabcd...

📋 Deployment Summary:
  Network: local
  Package ID: 0x1234...
  VK Object ID: 0xabcd...

Next steps:
  1. Update .env with:
  SUI_PACKAGE_ID=0x1234...
  SUI_VK_OBJECT_ID=0xabcd...
```

### 6. Update Configuration

Add the deployment info to your `.env` file:

```bash
# Network configuration
SUI_NETWORK=local  # or testnet, mainnet

# Deployment info (from deployment/{network}.toml)
SUI_PACKAGE_ID=0x1234...
SUI_VK_OBJECT_ID=0xabcd...

# Optional: Custom RPC URL
# SUI_RPC_URL=http://127.0.0.1:9000

# Optional: Gas budget (default: 0.1 SUI)
# SUI_GAS_BUDGET=100000000
```

### 7. Verify Deployment

```bash
# Check package info
sui client object <package-id>

# Check VK object
sui client object <vk-object-id>

# Run client with Sui integration
just run-sui local
```

## Network-Specific Examples

### Local Development
```bash
# Full workflow for local development
just sui-localnet  # Terminal 1

# Terminal 2:
cargo xtask sui keygen --alias local-dev
export SUI_ACTIVE_ALIAS=local-dev
sui client faucet

cd contracts/move
sui move build
sui client publish --gas-budget 10000000000 --with-unpublished-dependencies --json

# Save package ID to deployment/local.toml
cargo xtask sui setup --network local

# Update .env
echo "SUI_NETWORK=local" >> .env
echo "SUI_PACKAGE_ID=<package-id>" >> .env
echo "SUI_VK_OBJECT_ID=<vk-object-id>" >> .env
```

### Testnet Deployment
```bash
# Switch to testnet
sui client switch --env testnet

# Generate dedicated testnet address
cargo xtask sui keygen --alias testnet-deployer
export SUI_ACTIVE_ALIAS=testnet-deployer

# Fund from faucet
sui client faucet

# Deploy
cd contracts/move
sui move build
sui client publish --gas-budget 10000000000 --with-unpublished-dependencies --json

# Save package ID to deployment/testnet.toml
cargo xtask sui setup --network testnet

# Update .env for testnet
echo "SUI_NETWORK=testnet" >> .env
echo "SUI_PACKAGE_ID=<package-id>" >> .env
echo "SUI_VK_OBJECT_ID=<vk-object-id>" >> .env
```

### Mainnet Deployment
```bash
# Switch to mainnet
sui client switch --env mainnet

# Generate dedicated mainnet address (use hardware wallet in production!)
cargo xtask sui keygen --alias mainnet-deployer
export SUI_ACTIVE_ALIAS=mainnet-deployer

# Fund address (purchase SUI from exchange)
# Verify balance:
sui client gas

# Deploy with higher gas budget for mainnet
cd contracts/move
sui move build
sui client publish --gas-budget 20000000000 --with-unpublished-dependencies --json

# Save package ID to deployment/mainnet.toml
cargo xtask sui setup --network mainnet

# Update .env for mainnet
echo "SUI_NETWORK=mainnet" >> .env
echo "SUI_PACKAGE_ID=<package-id>" >> .env
echo "SUI_VK_OBJECT_ID=<vk-object-id>" >> .env
```

## Troubleshooting

### "Package dependency does not specify a published address"
**Solution:** Use `--with-unpublished-dependencies` flag when publishing.

### "Connection refused" to faucet
**Solution:** Ensure localnet was started with `--with-faucet` flag:
```bash
sui start --force-regenesis --with-faucet
```

### "Insufficient gas"
**Solution:** Increase `--gas-budget` or fund address with more SUI:
```bash
sui client faucet  # For local/testnet
```

### "No addresses in keystore"
**Solution:** Generate an address first:
```bash
cargo xtask sui keygen --alias my-address
# or
sui client new-address ed25519
```

### "Address with alias not found"
**Solution:** List available addresses and use correct alias:
```bash
sui client addresses
export SUI_ACTIVE_ALIAS=<correct-alias>
```

## Reference

### Gas Budget Guidelines
- **Local/Testnet:** 0.1-1 SUI (100,000,000 - 1,000,000,000 MIST)
- **Mainnet:** 1-10 SUI (1,000,000,000 - 10,000,000,000 MIST)
- Conversion: 1 SUI = 1,000,000,000 MIST

### Network URLs
- **Local:** `http://127.0.0.1:9000`
- **Testnet:** `https://fullnode.testnet.sui.io:443`
- **Mainnet:** `https://fullnode.mainnet.sui.io:443`

### Faucet URLs
- **Local:** `http://0.0.0.0:9123`
- **Testnet (CLI):** `sui client faucet`
- **Testnet (Web):** https://faucet.testnet.sui.io/

### Useful Commands
```bash
# View current environment
sui client envs

# View active address
sui client active-address

# View all addresses with aliases
sui client addresses

# View gas coins
sui client gas

# View all owned objects
sui client objects

# View specific object
sui client object <object-id>

# View transaction
sui client tx-block <tx-digest>
```

## Next Steps

After successful deployment:
1. Update `.env` with deployment info
2. Test contract interaction: `just run-sui local`
3. Create a game session on-chain
4. Submit proofs to verify functionality
5. For production: Consider using a hardware wallet for the deployer key
