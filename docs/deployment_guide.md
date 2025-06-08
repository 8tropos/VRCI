# Complete Deployment and Testing Guide

## Overview

This guide provides step-by-step instructions for building, deploying, and testing the w3pi token registry system on Paseo testnet. It covers everything from initial setup to advanced testing scenarios.

## Prerequisites

### Required Software

#### **1. Rust and Cargo**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

#### **2. WebAssembly Target**
```bash
# Add WebAssembly compilation target
rustup component add rust-src
rustup target add wasm32-unknown-unknown

# Verify target is installed
rustup target list --installed | grep wasm32
```

#### **3. cargo-contract**
```bash
# Install ink! contract tooling
cargo install --force --locked cargo-contract

# Verify installation
cargo contract --version
```

#### **4. pop-cli**
```bash
# Install Pop CLI for deployment
cargo install --git https://github.com/r0gue-io/pop-cli

# Verify installation
pop --version
```

### Network Setup

#### **Paseo Testnet Configuration**
- **RPC Endpoint**: `wss://rpc2.paseo.popnetwork.xyz`
- **Chain Type**: Paseo (Pop Network)
- **Currency**: PAS (Paseo tokens)
- **Faucet**: [Paseo Faucet](https://faucet.polkadot.io/)

#### **Account Setup**
You need a funded account on Paseo testnet. Choose one option:

**Option 1: Browser Wallet (Recommended)**
1. Install [Polkadot.js Extension](https://polkadot.js.org/extension/)
2. Create new account or import existing
3. Switch to Paseo testnet
4. Get test tokens from faucet

**Option 2: Development Account (Testing Only)**
```bash
# Use Alice's well-known test account
--suri "//Alice"
```

## Project Structure Setup

### **1. Create Project Directory**
```bash
# Create main project directory
mkdir w3pi && cd w3pi

# Create subdirectories
mkdir -p contracts/{shared,oracle,registry}/src
mkdir scripts
mkdir docs
```

### **2. Initialize Workspace**
Create the root `Cargo.toml`:

```toml
[workspace]
members = [
    "contracts/oracle", 
    "contracts/registry",
    # Note: shared is a dependency, not a workspace member
]
resolver = "2"

[profile.release]
codegen-units = 1
panic = "abort"
lto = true
opt-level = "z"

[profile.dev]
panic = "unwind"

[workspace.dependencies]
ink = { version = "5.1.0", default-features = false }
scale = { package = "parity-scale-codec", version = "3", default-features = false, features = ["derive"] }
scale-info = { version = "2", default-features = false, features = ["derive"] }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
pedantic = "warn"
```

### **3. Set Up Individual Contracts**

Copy the contract code from the previous artifacts:
- `contracts/shared/src/lib.rs` - Shared library code
- `contracts/oracle/src/lib.rs` - Oracle contract code  
- `contracts/registry/src/lib.rs` - Registry contract code

Copy the Cargo.toml files:
- `contracts/shared/Cargo.toml` - Shared library config
- `contracts/oracle/Cargo.toml` - Oracle contract config
- `contracts/registry/Cargo.toml` - Registry contract config

## Build Process

### **1. Build Order**
The build order is important due to dependencies:

```bash
# 1. Build shared library first (using regular cargo)
cd contracts/shared
cargo build
cargo test

# 2. Build oracle contract
cd ../oracle
pop build

# 3. Build registry contract (depends on shared)
cd ../registry  
pop build

# Return to project root
cd ../..
```

### **2. Verify Builds**
```bash
# Check that all contracts built successfully
ls -la contracts/oracle/target/ink/
ls -la contracts/registry/target/ink/

# Look for these files:
# - *.contract (bundled artifact)
# - *.wasm (compiled contract)
# - metadata.json (contract ABI)
```

### **3. Common Build Issues and Solutions**

#### **Issue: Shared library not found**
```bash
# Error: failed to load manifest for dependency `shared`
# Solution: Use correct path in contract Cargo.toml
shared = { path = "../shared", default-features = false, features = ["ink-as-dependency"] }
```

#### **Issue: Clippy warnings as errors**
```bash
# Error: arithmetic operation that can potentially result in unexpected side-effects
# Solution: Use saturating arithmetic (already fixed in our code)
self.next_token_id = self.next_token_id.saturating_add(1);
```

#### **Issue: Missing Default implementation**
```bash
# Warning: you should consider adding a `Default` implementation
# Solution: Already implemented in our contracts
impl Default for Oracle {
    fn default() -> Self {
        Self::new()
    }
}
```

## Deployment Process

### **1. Deploy Oracle Contract**

```bash
cd contracts/oracle

# Deploy with interactive wallet
pop up \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --constructor new \
  --use-wallet \
  --gas 500000000000 \
  --proof-size 2000000
```

**Expected Output:**
```
✅ Contract deployed and instantiated: 
● The contract address is "5HApKTfdHzpXnqrFHCuhSop1vDpZKWzV8jW4J4BLArJS1Dfc"
● The contract code hash is "0x871708ac27a2bf711926bbfcaf9903d7097d4354df8311959c2852b1aa5cb0d3"
```

**Save the contract address** - you'll need it for registry deployment.

### **2. Deploy Registry Contract**

```bash
cd ../registry

# Deploy registry contract
pop up \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --constructor new \
  --use-wallet \
  --gas 500000000000 \
  --proof-size 2000000
```

**Expected Output:**
```
✅ Contract deployed and instantiated:
● The contract address is "5CF56NywCHwv4a5AVdL6EhNAH69NBCenduZoApN7xxXgEhGc"
● The contract code hash is "0xf33f3e6af43e5f0e10a9cb6124cfba9b589f9be9e92a831682546a37baf988b3"
```

### **3. Record Contract Addresses**

Create a `.env` file to track your deployments:

```bash
# In project root
cat > .env << EOF
# w3pi Contract Addresses on Paseo Testnet
ORACLE_ADDRESS=5HApKTfdHzpXnqrFHCuhSop1vDpZKWzV8jW4J4BLArJS1Dfc
REGISTRY_ADDRESS=5CF56NywCHwv4a5AVdL6EhNAH69NBCenduZoApN7xxXgEhGc

# Network Configuration
RPC_URL=wss://rpc2.paseo.popnetwork.xyz
CHAIN=paseo

# Test Token (for demonstrations)
DUMMY_TOKEN=5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
EOF
```

## Testing Guide

### **1. Oracle Contract Testing**

#### **Test 1: Basic Owner Verification**
```bash
cd contracts/oracle

pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --message get_owner \
  --dry-run
```

**Expected Result:** Your account address

#### **Test 2: Read Non-Existent Price**
```bash
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --message get_price \
  --args $DUMMY_TOKEN \
  --dry-run
```

**Expected Result:** `Ok(None)` (no price data yet)

#### **Test 3: Set Price Data**
```bash
# Interactive mode - pop will prompt for parameters
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --dry-run
# Select: update_price
# Token: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY  
# Price: 10000000000 (1 DOT in plancks)
# Execute: Yes
```

**Expected Events:**
- `ContractEmitted` - PriceUpdated event
- `ExtrinsicSuccess` - Transaction succeeded

#### **Test 4: Verify Price Was Set**
```bash
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --message get_price \
  --args $DUMMY_TOKEN \
  --dry-run
```

**Expected Result:** `Ok(Some(10000000000))`

#### **Test 5: Set Market Data**
```bash
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --message update_market_data \
  --args $DUMMY_TOKEN 1000000000000000 100000000000000 \
  --use-wallet \
  --execute
```

**Parameters Explained:**
- Token: `$DUMMY_TOKEN`
- Market Cap: `1000000000000000` (100,000 DOT)
- Volume: `100000000000000` (10,000 DOT)

#### **Test 6: Verify Market Data**
```bash
# Test market cap
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --message get_market_cap \
  --args $DUMMY_TOKEN \
  --dry-run

# Test volume
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --message get_market_volume \
  --args $DUMMY_TOKEN \
  --dry-run
```

### **2. Registry Contract Testing**

#### **Test 1: Initial State**
```bash
cd ../registry

# Check token count (should be 0)
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message get_token_count \
  --dry-run

# Check owner
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message get_owner \
  --dry-run
```

#### **Test 2: Add Token to Registry**
```bash
# Interactive mode
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --dry-run
# Select: add_token
# token_contract: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
# oracle_contract: 5HApKTfdHzpXnqrFHCuhSop1vDpZKWzV8jW4J4BLArJS1Dfc
# Execute: Yes
```

**Expected Events:**
- `ContractEmitted` - TokenAdded event
- `ExtrinsicSuccess` - Transaction succeeded

#### **Test 3: Verify Token Was Added**
```bash
# Check token count (should be 1)
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message get_token_count \
  --dry-run

# Check if token exists
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message token_exists \
  --args 1 \
  --dry-run
```

#### **Test 4: Cross-Contract Call (The Main Event!)**
```bash
# Get enriched token data with live oracle prices
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message get_token_data \
  --args 1 \
  --dry-run
```

**Expected Result:**
```
Ok(Ok(EnrichedTokenData {
    token_contract: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY,
    oracle_contract: 5HApKTfdHzpXnqrFHCuhSop1vDpZKWzV8jW4J4BLArJS1Dfc,
    balance: 0,
    weight_investment: 0,
    tier: 0,
    market_cap: 1000000000000000,   // From oracle
    market_volume: 100000000000000, // From oracle  
    price: 10000000000              // From oracle
}))
```

#### **Test 5: Update Registry Data**
```bash
# Update token metadata
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message update_token \
  --args 1 500000000000 7500 4 \
  --use-wallet \
  --execute
```

**Parameters Explained:**
- Token ID: `1`
- Balance: `500000000000` (50 DOT)
- Weight: `7500` (75% allocation in basis points)
- Tier: `4` (low risk tier)

#### **Test 6: Verify Updated Data**
```bash
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message get_token_data \
  --args 1 \
  --dry-run
```

**Expected:** Same as before but with updated balance, weight, and tier.

### **3. End-to-End Testing Scenarios**

#### **Scenario 1: Price Update Propagation**

```bash
# 1. Update oracle price
cd contracts/oracle
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --message update_price \
  --args $DUMMY_TOKEN 15000000000 \
  --use-wallet \
  --execute

# 2. Check registry reflects new price
cd ../registry
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message get_token_data \
  --args 1 \
  --dry-run
```

**Expected:** Price should now be `15000000000` (1.5 DOT)

#### **Scenario 2: Multiple Token Management**

```bash
# Add second token with same oracle
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message add_token \
  --args 5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUpnhM $ORACLE_ADDRESS \
  --use-wallet \
  --execute

# Set price for second token in oracle
cd ../oracle
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --message update_price \
  --args 5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUpnhM 25000000000 \
  --use-wallet \
  --execute

# Check both tokens in registry
cd ../registry
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message get_token_data \
  --args 1 \
  --dry-run

pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message get_token_data \
  --args 2 \
  --dry-run
```

#### **Scenario 3: Error Handling**

```bash
# Test non-existent token
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $REGISTRY_ADDRESS \
  --message get_token_data \
  --args 999 \
  --dry-run
```

**Expected:** `Err(TokenNotFound)`

```bash
# Test unauthorized access (using different account)
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $ORACLE_ADDRESS \
  --message update_price \
  --args $DUMMY_TOKEN 20000000000 \
  --suri "//Bob" \
  --execute
```

**Expected:** Transaction should fail with `Unauthorized` error

## Automation Scripts

### **1. Deployment Script**

Create `scripts/deploy.sh`:

```bash
#!/bin/bash
set -e

echo "🚀 Starting w3pi deployment to Paseo testnet..."

# Load environment variables
source .env 2>/dev/null || true

# Build contracts
echo "📦 Building contracts..."
cd contracts/shared && cargo build
cd ../oracle && pop build  
cd ../registry && pop build
cd ../..

# Deploy Oracle
echo "🔮 Deploying Oracle contract..."
cd contracts/oracle
ORACLE_OUTPUT=$(pop up \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --constructor new \
  --use-wallet \
  --gas 500000000000 \
  --proof-size 2000000)

ORACLE_ADDRESS=$(echo "$ORACLE_OUTPUT" | grep -o '5[A-Za-z0-9]\{47\}' | head -1)
echo "✅ Oracle deployed at: $ORACLE_ADDRESS"

# Deploy Registry
echo "📋 Deploying Registry contract..."
cd ../registry
REGISTRY_OUTPUT=$(pop up \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --constructor new \
  --use-wallet \
  --gas 500000000000 \
  --proof-size 2000000)

REGISTRY_ADDRESS=$(echo "$REGISTRY_OUTPUT" | grep -o '5[A-Za-z0-9]\{47\}' | head -1)
echo "✅ Registry deployed at: $REGISTRY_ADDRESS"

# Update .env file
cd ../..
echo "ORACLE_ADDRESS=$ORACLE_ADDRESS" > .env
echo "REGISTRY_ADDRESS=$REGISTRY_ADDRESS" >> .env
echo "RPC_URL=wss://rpc2.paseo.popnetwork.xyz" >> .env
echo "DUMMY_TOKEN=5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" >> .env

echo "🎉 Deployment complete!"
echo "📝 Contract addresses saved to .env file"
```

### **2. Testing Script**

Create `scripts/test.sh`:

```bash
#!/bin/bash
set -e

# Load environment variables
source .env

echo "🧪 Starting comprehensive testing..."

# Test Oracle
echo "🔮 Testing Oracle contract..."
cd contracts/oracle

# Set price data
pop call contract \
  --url $RPC_URL \
  --contract $ORACLE_ADDRESS \
  --message update_price \
  --args $DUMMY_TOKEN 10000000000 \
  --use-wallet \
  --execute

# Verify price
PRICE_RESULT=$(pop call contract \
  --url $RPC_URL \
  --contract $ORACLE_ADDRESS \
  --message get_price \
  --args $DUMMY_TOKEN \
  --dry-run)

echo "Oracle price result: $PRICE_RESULT"

# Test Registry
echo "📋 Testing Registry contract..."
cd ../registry

# Add token
pop call contract \
  --url $RPC_URL \
  --contract $REGISTRY_ADDRESS \
  --message add_token \
  --args $DUMMY_TOKEN $ORACLE_ADDRESS \
  --use-wallet \
  --execute

# Test cross-contract call
TOKEN_DATA=$(pop call contract \
  --url $RPC_URL \
  --contract $REGISTRY_ADDRESS \
  --message get_token_data \
  --args 1 \
  --dry-run)

echo "Cross-contract call result: $TOKEN_DATA"

echo "✅ All tests completed!"
cd ../..
```

### **3. Interaction Script**

Create `scripts/interact.sh`:

```bash
#!/bin/bash

# Interactive script for common operations
source .env

echo "🎮 w3pi Contract Interaction Menu"
echo "================================"
echo "1. Update Oracle Price"
echo "2. Get Token Data"
echo "3. Add New Token"
echo "4. Update Token Info"
echo "5. Get Token Count"
echo "6. Exit"

read -p "Select option (1-6): " choice

case $choice in
    1)
        read -p "Enter token address: " token
        read -p "Enter price in plancks: " price
        cd contracts/oracle
        pop call contract \
          --url $RPC_URL \
          --contract $ORACLE_ADDRESS \
          --message update_price \
          --args $token $price \
          --use-wallet \
          --execute
        ;;
    2)
        read -p "Enter token ID: " token_id
        cd contracts/registry
        pop call contract \
          --url $RPC_URL \
          --contract $REGISTRY_ADDRESS \
          --message get_token_data \
          --args $token_id \
          --dry-run
        ;;
    3)
        read -p "Enter token contract address: " token_contract
        read -p "Enter oracle contract address: " oracle_contract
        cd contracts/registry
        pop call contract \
          --url $RPC_URL \
          --contract $REGISTRY_ADDRESS \
          --message add_token \
          --args $token_contract $oracle_contract \
          --use-wallet \
          --execute
        ;;
    4)
        read -p "Enter token ID: " token_id
        read -p "Enter balance: " balance
        read -p "Enter weight (0-10000): " weight
        read -p "Enter tier (0-5): " tier
        cd contracts/registry
        pop call contract \
          --url $RPC_URL \
          --contract $REGISTRY_ADDRESS \
          --message update_token \
          --args $token_id $balance $weight $tier \
          --use-wallet \
          --execute
        ;;
    5)
        cd contracts/registry
        pop call contract \
          --url $RPC_URL \
          --contract $REGISTRY_ADDRESS \
          --message get_token_count \
          --dry-run
        ;;
    6)
        echo "Goodbye!"
        exit 0
        ;;
    *)
        echo "Invalid option"
        ;;
esac
```

Make scripts executable:
```bash
chmod +x scripts/*.sh
```

## Advanced Testing

### **1. Performance Testing**

#### **Gas Usage Analysis**
```bash
# Test gas consumption for different operations
cd contracts/oracle

# Dry run to see gas estimates
pop call contract \
  --url $RPC_URL \
  --contract $ORACLE_ADDRESS \
  --message update_price \
  --args $DUMMY_TOKEN 10000000000 \
  --dry-run \
  --verbose
```

#### **Cross-Contract Call Performance**
```bash
# Compare gas usage: basic vs enriched calls
cd contracts/registry

# Basic call (no cross-contract)
pop call contract \
  --url $RPC_URL \
  --contract $REGISTRY_ADDRESS \
  --message get_basic_token_data \
  --args 1 \
  --dry-run

# Enriched call (with cross-contract)
pop call contract \
  --url $RPC_URL \
  --contract $REGISTRY_ADDRESS \
  --message get_token_data \
  --args 1 \
  --dry-run
```

### **2. Stress Testing**

#### **Multiple Token Registration**
```bash
#!/bin/bash
# Test script for registering multiple tokens

source .env

echo "🔥 Stress testing with multiple tokens..."

# Array of test token addresses
TOKENS=(
    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
    "5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUpnhM" 
    "5FbSD6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy"
    "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy"
    "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw"
)

cd contracts/registry

for i in "${!TOKENS[@]}"; do
    echo "Registering token $((i+1)): ${TOKENS[$i]}"
    
    pop call contract \
      --url $RPC_URL \
      --contract $REGISTRY_ADDRESS \
      --message add_token \
      --args ${TOKENS[$i]} $ORACLE_ADDRESS \
      --use-wallet \
      --execute
    
    sleep 2  # Avoid rate limiting
done

echo "✅ Registered ${#TOKENS[@]} tokens"

# Test retrieval of all tokens
for i in $(seq 1 ${#TOKENS[@]}); do
    echo "Getting data for token $i..."
    pop call contract \
      --url $RPC_URL \
      --contract $REGISTRY_ADDRESS \
      --message get_token_data \
      --args $i \
      --dry-run
done
```

### **3. Error Condition Testing**

#### **Authorization Testing**
```bash
# Test unauthorized access attempts
echo "🔒 Testing authorization controls..."

# Try to update oracle as non-owner (should fail)
pop call contract \
  --url $RPC_URL \
  --contract $ORACLE_ADDRESS \
  --message update_price \
  --args $DUMMY_TOKEN 20000000000 \
  --suri "//Bob" \
  --execute

# Try to add token to registry as non-owner (should fail)
pop call contract \
  --url $RPC_URL \
  --contract $REGISTRY_ADDRESS \
  --message add_token \
  --args $DUMMY_TOKEN $ORACLE_ADDRESS \
  --suri "//Charlie" \
  --execute
```

#### **Boundary Testing**
```bash
# Test edge cases and boundary conditions

# Test with maximum values
pop call contract \
  --url $RPC_URL \
  --contract $ORACLE_ADDRESS \
  --message update_price \
  --args $DUMMY_TOKEN 340282366920938463463374607431768211455 \
  --dry-run

# Test registry with maximum weight (10000 basis points = 100%)
pop call contract \
  --url $RPC_URL \
  --contract $REGISTRY_ADDRESS \
  --message update_token \
  --args 1 1000000000000 10000 5 \
  --dry-run

# Test with invalid token ID
pop call contract \
  --url $RPC_URL \
  --contract $REGISTRY_ADDRESS \
  --message get_token_data \
  --args 999999 \
  --dry-run
```

## Monitoring and Logging

### **1. Event Monitoring**

#### **Set Up Event Watching**
```bash
# Monitor oracle events
echo "👀 Monitoring oracle events..."

# Use substrate API tools or custom scripts
wscat -c $RPC_URL
# Then send subscription request for contract events
```

#### **Event Analysis Script**
```bash
#!/bin/bash
# Parse and analyze contract events

LOG_FILE="contract_events.log"

echo "📊 Analyzing contract events..."

# Count event types
echo "Event Summary:"
echo "=============="
grep "PriceUpdated" $LOG_FILE | wc -l | xargs echo "Price Updates:"
grep "TokenAdded" $LOG_FILE | wc -l | xargs echo "Tokens Added:"
grep "TokenUpdated" $LOG_FILE | wc -l | xargs echo "Token Updates:"

# Show recent events
echo ""
echo "Recent Events:"
echo "=============="
tail -10 $LOG_FILE
```

### **2. Health Checks**

#### **Contract Health Script**
```bash
#!/bin/bash
# Check contract health and accessibility

source .env

echo "🏥 Running health checks..."

# Test Oracle accessibility
echo "Checking Oracle..."
ORACLE_OWNER=$(pop call contract \
  --url $RPC_URL \
  --contract $ORACLE_ADDRESS \
  --message get_owner \
  --dry-run 2>/dev/null)

if [[ $? -eq 0 ]]; then
    echo "✅ Oracle is accessible"
else
    echo "❌ Oracle is not accessible"
fi

# Test Registry accessibility  
echo "Checking Registry..."
TOKEN_COUNT=$(pop call contract \
  --url $RPC_URL \
  --contract $REGISTRY_ADDRESS \
  --message get_token_count \
  --dry-run 2>/dev/null)

if [[ $? -eq 0 ]]; then
    echo "✅ Registry is accessible"
    echo "📊 Token count: $TOKEN_COUNT"
else
    echo "❌ Registry is not accessible"
fi

# Test cross-contract functionality
echo "Checking cross-contract calls..."
if [[ $(echo "$TOKEN_COUNT" | grep -o '[0-9]*') -gt 0 ]]; then
    CROSS_CALL=$(pop call contract \
      --url $RPC_URL \
      --contract $REGISTRY_ADDRESS \
      --message get_token_data \
      --args 1 \
      --dry-run 2>/dev/null)
    
    if [[ $? -eq 0 ]]; then
        echo "✅ Cross-contract calls working"
    else
        echo "❌ Cross-contract calls failing"
    fi
else
    echo "⚠️  No tokens registered for cross-contract testing"
fi

echo "Health check complete!"
```

## Troubleshooting Guide

### **Common Issues and Solutions**

#### **Issue 1: Build Failures**

**Problem:** `error: linking with 'rust-lld' failed`
```bash
# Solution: Clean and rebuild
cargo clean
pop build
```

**Problem:** `shared` dependency not found
```bash
# Solution: Check path in Cargo.toml
shared = { path = "../shared", default-features = false, features = ["ink-as-dependency"] }
```

#### **Issue 2: Deployment Failures**

**Problem:** `Insufficient balance`
```bash
# Solution: Get more test tokens from faucet
# Visit: https://faucet.polkadot.io/
```

**Problem:** `Connection refused`
```bash
# Solution: Check RPC endpoint
--url wss://rpc2.paseo.popnetwork.xyz
```

#### **Issue 3: Contract Call Failures**

**Problem:** `ContractTrapped`
```bash
# Solution: Check cross-contract addresses are correct
# Verify oracle contract is deployed and accessible
pop call contract --contract $ORACLE_ADDRESS --message get_owner --dry-run
```

**Problem:** `Invalid message name`
```bash
# Solution: Run call from correct contract directory
cd contracts/oracle  # For oracle calls
cd contracts/registry # For registry calls
```

#### **Issue 4: Cross-Contract Call Issues**

**Problem:** Oracle returns `None` values
```bash
# Solution: Ensure oracle has data for the token
pop call contract \
  --contract $ORACLE_ADDRESS \
  --message get_price \
  --args $TOKEN_ADDRESS \
  --dry-run
```

**Problem:** Registry can't call oracle
```bash
# Solution: Verify addresses are correct in registry
# Check that oracle is deployed and responding
```

### **Debug Commands**

#### **Verify Contract State**
```bash
# Check oracle state
pop call contract --contract $ORACLE_ADDRESS --message get_owner --dry-run

# Check registry state  
pop call contract --contract $REGISTRY_ADDRESS --message get_token_count --dry-run

# Check specific token data
pop call contract --contract $REGISTRY_ADDRESS --message token_exists --args 1 --dry-run
```

#### **Network Debugging**
```bash
# Test RPC connection
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
  https://rpc2.paseo.popnetwork.xyz

# Check account balance
pop call contract --dry-run  # Will show your account info
```

## Production Considerations

### **Security Checklist**

- [ ] **Owner Security**: Use hardware wallet or multisig for contract ownership
- [ ] **Access Control**: Verify only authorized accounts can update data
- [ ] **Input Validation**: Test with edge cases and invalid inputs
- [ ] **Oracle Security**: Ensure oracle data sources are reliable
- [ ] **Gas Limits**: Set appropriate limits for all operations
- [ ] **Error Handling**: Test all error conditions
- [ ] **Monitoring**: Set up alerts for critical events

### **Performance Optimization**

- [ ] **Gas Efficiency**: Optimize contract calls for lower gas usage
- [ ] **Batch Operations**: Group multiple calls when possible
- [ ] **Caching**: Cache frequently accessed data
- [ ] **Rate Limiting**: Implement protection against spam
- [ ] **Load Testing**: Test with high transaction volumes

### **Maintenance Planning**

- [ ] **Upgrade Strategy**: Plan for contract upgrades
- [ ] **Data Migration**: Prepare for schema changes
- [ ] **Backup Procedures**: Regular state snapshots
- [ ] **Documentation**: Keep deployment records updated
- [ ] **Team Training**: Ensure team knows operational procedures

## Next Steps

### **Immediate Actions**
1. Deploy contracts to Paseo testnet
2. Run basic functionality tests
3. Verify cross-contract calls work
4. Document your specific contract addresses

### **Development Extensions**
1. Add more oracle data sources
2. Implement governance mechanisms
3. Create frontend interface
4. Add automated price feeds
5. Implement advanced portfolio features

### **Production Preparation**
1. Security audit
2. Comprehensive testing
3. Performance optimization
4. Monitoring setup
5. Team training

## Support and Resources

### **Documentation**
- [ink! Documentation](https://use.ink/)
- [Pop CLI Documentation](https://learn.onpop.io/)
- [Polkadot.js Documentation](https://polkadot.js.org/docs/)

### **Community**
- [ink! Discord](https://discord.gg/wGUDt2p)
- [Polkadot Discord](https://discord.gg/polkadot)
- [Substrate Stack Exchange](https://substrate.stackexchange.com/)

### **Tools**
- [Contracts UI](https://contracts-ui.substrate.io/)
- [Polkadot.js Apps](https://polkadot.js.org/apps/)
- [Subscan Explorer](https://polkadot.subscan.io/)

This guide provides everything your team needs to successfully deploy and test the w3pi token registry system. Follow the steps sequentially, and you'll have a working cross-contract system demonstrating advanced ink! v5 capabilities!