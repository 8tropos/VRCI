# W3PI Token Contract - Developer Guide

## Overview

The W3PI Token contract is a fully compliant PSP22 (Polkadot Standard Proposal 22) fungible token implementation built with ink! v5. It provides standard ERC20-like functionality with additional features for the W3PI ecosystem including metadata support, minting/burning capabilities, and seamless integration with the oracle and registry contracts.

**Deployed Address:** `5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr`

## Key Features

### ✅ PSP22 Standard Compliance
- Transfer functionality with allowance mechanism
- Event emission for all state changes
- Full compatibility with PSP22 ecosystem

### ✅ Metadata Support
- Token name: "W3PI Token"
- Symbol: "W3PI"
- Decimals: 12 (supporting micro-transactions)
- Optional metadata fields

### ✅ Advanced Token Economics
- Fixed supply model (1,000,000 tokens)
- Precision handling with 12 decimal places
- Overflow-safe arithmetic operations

### ✅ Developer-Friendly Architecture
- Modular design with separated concerns
- Comprehensive error handling
- Extensive unit test coverage

## Contract Functions

### Core PSP22 Functions

#### `total_supply() -> u128`
Returns the total token supply.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message total_supply \
  --dry-run
```

#### `balance_of(owner: AccountId) -> u128`
Returns the balance of a specific account.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message balance_of \
  --args YOUR_ACCOUNT_ADDRESS \
  --dry-run
```

#### `transfer(to: AccountId, value: u128, data: Vec<u8>) -> Result<(), PSP22Error>`
Transfers tokens from caller to another account.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message transfer \
  --args RECIPIENT_ADDRESS 1000000000000 [] \
  --use-wallet \
  --execute
```

#### `approve(spender: AccountId, value: u128) -> Result<(), PSP22Error>`
Approves another account to spend tokens on your behalf.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message approve \
  --args SPENDER_ADDRESS 500000000000 \
  --use-wallet \
  --execute
```

#### `allowance(owner: AccountId, spender: AccountId) -> u128`
Returns the remaining allowance between owner and spender.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message allowance \
  --args OWNER_ADDRESS SPENDER_ADDRESS \
  --dry-run
```

#### `transfer_from(from: AccountId, to: AccountId, value: u128, data: Vec<u8>) -> Result<(), PSP22Error>`
Transfers tokens on behalf of another account (requires prior approval).
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message transfer_from \
  --args FROM_ADDRESS TO_ADDRESS 250000000000 [] \
  --use-wallet \
  --execute
```

### Allowance Management

#### `increase_allowance(spender: AccountId, delta_value: u128) -> Result<(), PSP22Error>`
Safely increases allowance to prevent race conditions.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message increase_allowance \
  --args SPENDER_ADDRESS 100000000000 \
  --use-wallet \
  --execute
```

#### `decrease_allowance(spender: AccountId, delta_value: u128) -> Result<(), PSP22Error>`
Safely decreases allowance.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message decrease_allowance \
  --args SPENDER_ADDRESS 50000000000 \
  --use-wallet \
  --execute
```

### Metadata Functions

#### `token_name() -> Option<String>`
Returns the token name.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message token_name \
  --dry-run
```

#### `token_symbol() -> Option<String>`
Returns the token symbol.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message token_symbol \
  --dry-run
```

#### `token_decimals() -> u8`
Returns the number of decimals.
```bash
pop call contract \
  --contract 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr \
  --message token_decimals \
  --dry-run
```

## Events

### Transfer Event
Emitted when tokens are transferred, minted, or burned.
```rust
pub struct Transfer {
    pub from: Option<AccountId>,  // None for minting
    pub to: Option<AccountId>,    // None for burning
    pub value: u128,
}
```

### Approval Event
Emitted when allowance is set or modified.
```rust
pub struct Approval {
    pub owner: AccountId,
    pub spender: AccountId,
    pub amount: u128,
}
```

## Error Types

```rust
pub enum PSP22Error {
    Custom(String),                    // Custom implementation errors
    InsufficientBalance,              // Not enough tokens
    InsufficientAllowance,            // Not enough allowance
    ZeroRecipientAddress,             // [deprecated]
    ZeroSenderAddress,                // [deprecated]
    SafeTransferCheckFailed(String),  // [deprecated]
}
```

## Deployment Guide

### Prerequisites

1. **Install Dependencies**
```bash
# Install Rust and cargo-contract
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install --force --locked cargo-contract
cargo install --git https://github.com/r0gue-io/pop-cli

# Add WebAssembly target
rustup target add wasm32-unknown-unknown
```

2. **Fund Your Account**
- Get PAS tokens from [Paseo Faucet](https://faucet.polkadot.io/)
- Ensure you have at least 10 PAS for deployment

### Build Process

```bash
# Clone project and navigate to token contract
cd contracts/token

# Build the contract
pop build

# Verify build artifacts
ls -la target/ink/
# Should show: token.contract, token.wasm, metadata.json
```

### Deployment Options

#### Option 1: Interactive Deployment (Recommended)
```bash
pop up \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --constructor new \
  --use-wallet \
  --gas 500000000000 \
  --proof-size 2000000
```

When prompted, enter:
- **supply**: `1000000000000` (1M tokens with 12 decimals)
- **name**: `Your Token Name`
- **symbol**: `SYMBOL`
- **decimals**: `12`

#### Option 2: Command Line Deployment
```bash
pop up \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --constructor new \
  --args 1000000000000 \"Your\ Token\ Name\" \"SYMBOL\" 12 \
  --use-wallet \
  --gas 500000000000 \
  --proof-size 2000000
```

#### Option 3: Minimal Token (No Metadata)
```bash
pop up \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --constructor new \
  --args 1000000000000 None None 12 \
  --use-wallet \
  --gas 500000000000 \
  --proof-size 2000000
```

### Deployment Parameters Explained

- **supply**: Total token supply (including decimals)
  - Example: `1000000000000` = 1M tokens with 12 decimals
- **name**: Human-readable token name (optional)
- **symbol**: Short ticker symbol (optional)
- **decimals**: Number of decimal places (0-18, typically 12 or 18)

### Gas Configuration

- **Gas Limit**: `500000000000` (500B units)
- **Proof Size**: `2000000` (2MB)
- **Network**: Paseo Testnet

## Integration Examples

### Basic Transfer Example
```bash
#!/bin/bash
TOKEN_ADDRESS="5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr"
RECIPIENT="5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
AMOUNT="1000000000000"  # 1 token with 12 decimals

# Transfer tokens
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $TOKEN_ADDRESS \
  --message transfer \
  --args $RECIPIENT $AMOUNT [] \
  --use-wallet \
  --execute
```

### Allowance Pattern Example
```bash
#!/bin/bash
TOKEN_ADDRESS="5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr"
SPENDER="5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUpnhM"
AMOUNT="500000000000"   # 0.5 tokens

# 1. Approve spender
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $TOKEN_ADDRESS \
  --message approve \
  --args $SPENDER $AMOUNT \
  --use-wallet \
  --execute

# 2. Check allowance
pop call contract \
  --url wss://rpc2.paseo.popnetwork.xyz \
  --contract $TOKEN_ADDRESS \
  --message allowance \
  --args YOUR_ADDRESS $SPENDER \
  --dry-run
```

## Integration with W3PI Ecosystem

### Oracle Price Feeds
The token can be integrated with the W3PI Oracle contract for real-time price data:

```bash
# Set price in oracle
pop call contract \
  --contract ORACLE_ADDRESS \
  --message update_price \
  --args 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr 15000000000 \
  --use-wallet \
  --execute
```

### Registry Integration
Add token to the W3PI Registry for portfolio management:

```bash
# Add token to registry
pop call contract \
  --contract REGISTRY_ADDRESS \
  --message add_token \
  --args 5DNvZmAA6QwqYdvBFhcdh8fd3U9iyqrUAqh798SxUDfA1fnr ORACLE_ADDRESS \
  --use-wallet \
  --execute
```

## Testing Guide

### Unit Tests
The contract includes comprehensive unit tests:

```bash
cd contracts/token
cargo test
```

### Integration Testing
```bash
# Test basic functionality
./scripts/test_token.sh

# Test with other contracts
./scripts/test_ecosystem.sh
```

### Manual Testing Checklist

- [ ] Deploy contract successfully
- [ ] Verify metadata (name, symbol, decimals)
- [ ] Check initial supply and creator balance
- [ ] Test transfer functionality
- [ ] Test approval/allowance mechanism
- [ ] Test transfer_from functionality
- [ ] Verify event emission
- [ ] Test error conditions (insufficient balance, etc.)

## Best Practices

### Security Considerations

1. **Overflow Protection**: All arithmetic uses saturating operations
2. **Reentrancy Safety**: No external calls during state changes
3. **Access Control**: No privileged functions (immutable supply)
4. **Input Validation**: Comprehensive parameter checking

### Gas Optimization

1. **Storage Efficiency**: Mappings are removed when balance/allowance is zero
2. **Event Optimization**: No-op operations don't emit events
3. **Batch Operations**: Consider using allowance for multiple transfers

### Development Tips

1. **Use Dry Runs**: Always test with `--dry-run` first
2. **Monitor Events**: Watch for Transfer/Approval events
3. **Handle Errors**: Implement proper error handling in dApps
4. **Decimal Precision**: Remember to account for 12 decimals in calculations

## Common Issues and Solutions

### Issue: String Parameter Errors
```
Error: Expected a String value
```
**Solution**: Use interactive mode or escape quotes properly:
```bash
--args 1000000000000 \"Token\ Name\" \"SYMBOL\" 12
```

### Issue: Insufficient Gas
```
Error: Insufficient gas
```
**Solution**: Increase gas limit:
```bash
--gas 1000000000000
```

### Issue: Balance Calculation
```
Error: Balance shows large numbers
```
**Solution**: Remember to account for decimals:
```
Displayed: 1000000000000
Actual: 1.000000000000 tokens (12 decimals)
```

## Contract Architecture

### Modular Design
```
token/
├── lib.rs          # Main contract logic
├── data.rs         # PSP22Data implementation
├── traits.rs       # PSP22 trait definitions  
├── events.rs       # Event definitions
├── errors.rs       # Error types
└── testing.rs      # Test utilities
```

### Dependencies
- `ink`: Core smart contract framework
- `shared`: Common types and utilities
- `scale`: Serialization codec
- `scale-info`: Type information

## Version Information

- **Contract Version**: 0.1.0
- **ink! Version**: 5.1.0
- **Substrate Compatibility**: Latest
- **Network**: Paseo Testnet

## Support and Resources

### Documentation
- [PSP22 Standard](https://github.com/w3f/PSPs/blob/master/PSPs/psp-22.md)
- [ink! Documentation](https://use.ink/)
- [Pop CLI Guide](https://learn.onpop.io/)

### Community
- [ink! Discord](https://discord.gg/wGUDt2p)
- [Polkadot Discord](https://discord.gg/polkadot)

### Tools
- [Contracts UI](https://contracts-ui.substrate.io/)
- [Polkadot.js Apps](https://polkadot.js.org/apps/)

---

*This token contract provides the foundation for the W3PI ecosystem, enabling decentralized token management with oracle-fed pricing and registry-based portfolio tracking.*