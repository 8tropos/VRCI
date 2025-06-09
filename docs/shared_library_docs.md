# Shared Library Documentation

## Overview

The Shared library (`contracts/shared`) provides common types, traits, and utilities used across all contracts in the w3pi ecosystem. It ensures type consistency, reduces code duplication, and provides a foundation for cross-contract interactions.

## Purpose

### **Type Consistency**

- Shared data structures used across contracts
- Common error types for uniform error handling
- Standardized traits for contract interfaces

### **Code Reuse**

- Eliminates duplicate type definitions
- Provides common functionality
- Ensures consistent behavior across contracts

### **Cross-Contract Compatibility**

- Enables type-safe cross-contract calls
- Provides contract interface definitions
- Facilitates ecosystem integration

## Architecture

```
┌─────────────────┐
│ Shared Library  │
├─────────────────┤
│ • TokenData     │
│ • Error Types   │
│ • Oracle Trait  │
│ • Constants     │
└─────────────────┘
         ↑
    Used by all contracts
         ↓
┌─────────────────┐     ┌─────────────────┐
│ Oracle Contract │     │Registry Contract│
│                 │     │                 │
│ • Implements    │     │ • Uses TokenData│
│   Oracle trait  │     │ • Uses Errors   │
│ • Uses Errors   │     │ • Calls Oracle  │
└─────────────────┘     └─────────────────┘
```

## Core Components

### Data Structures

#### TokenData

The fundamental data structure representing a token in the registry.

```rust
#[derive(Decode, Encode, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct TokenData {
    /// Token contract address
    pub token_contract: AccountId,
    /// Oracle contract address
    pub oracle_contract: AccountId,
    /// Current balance (managed by registry)
    pub balance: u128,
    /// Investment weight (0-10000 for basis points)
    pub weight_investment: u32,
    /// Token tier (0-5, where 5 is highest tier)
    pub tier: u32,
}
```

**Field Descriptions:**

- `token_contract`: The on-chain address of the actual token contract
- `oracle_contract`: The address of the oracle providing price data
- `balance`: Amount held in the registry (in plancks)
- `weight_investment`: Portfolio weight in basis points (100 = 1%, 10000 = 100%)
- `tier`: Quality/risk tier (0 = highest risk, 5 = lowest risk)

#### EnrichedTokenData

Extended token data that includes live oracle information.

```rust
#[derive(Decode, Encode, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct EnrichedTokenData {
    pub token_contract: AccountId,
    pub oracle_contract: AccountId,
    pub balance: u128,
    pub weight_investment: u32,
    pub tier: u32,
    /// Market cap in plancks
    pub market_cap: u128,
    /// 24h trading volume in plancks
    pub market_volume: u128,
    /// Current price in plancks
    pub price: u128,
}
```

**Additional Fields:**

- `market_cap`: Total market capitalization from oracle
- `market_volume`: 24-hour trading volume from oracle
- `price`: Current token price from oracle

### Error Types

#### Error Enum

Centralized error handling for all contracts.

```rust
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    Unauthorized,
    TokenNotFound,
    OracleCallFailed,
    InvalidParameter,
    InsufficientBalance,
}
```

**Error Descriptions:**

- `Unauthorized`: Caller lacks permission for the operation
- `TokenNotFound`: Requested token ID doesn't exist
- `OracleCallFailed`: Cross-contract call to oracle failed
- `InvalidParameter`: Invalid input parameter provided
- `InsufficientBalance`: Insufficient balance for operation

### Traits

#### Oracle Trait

Defines the interface that oracle contracts must implement.

```rust
#[ink::trait_definition]
pub trait Oracle {
    /// Get the current price of a token in plancks
    #[ink(message)]
    fn get_price(&self, token: AccountId) -> Option<u128>;

    /// Get the market cap of a token in plancks
    #[ink(message)]
    fn get_market_cap(&self, token: AccountId) -> Option<u128>;

    /// Get the market volume of a token in plancks
    #[ink(message)]
    fn get_market_volume(&self, token: AccountId) -> Option<u128>;
}
```

**Usage Benefits:**

- **Type Safety**: Ensures oracle contracts implement required methods
- **Documentation**: Clear interface contract
- **Future Compatibility**: Easy to extend with new methods

## Dependencies and Features

### Cargo.toml Configuration

```toml
[package]
name = "shared"
version = "0.1.0"
authors = ["3dln <ashcan@3dln.com>"]
edition = "2021"

[dependencies]
ink = { workspace = true, default-features = false }
scale = { workspace = true, default-features = false }
scale-info = { workspace = true, default-features = false }

[lib]
name = "shared"
path = "src/lib.rs"

[features]
default = ["std"]
std = ["ink/std", "scale/std", "scale-info/std"]
ink-as-dependency = []
```

### Feature Flags

#### `std` Feature

- **Purpose**: Standard library support for off-chain environments
- **Enables**: Full Rust std library, debugging, testing
- **Usage**: Development, testing, metadata generation

#### `ink-as-dependency` Feature

- **Purpose**: Allows other contracts to import this as a dependency
- **Required**: When other contracts use `use shared::*`
- **Usage**: Cross-contract integration

## Type Annotations and Serialization

### SCALE Codec Support

All types implement SCALE (Simple Concatenated Aggregate Little-Endian) codec for blockchain serialization.

```rust
#[derive(Decode, Encode, Clone, Debug, PartialEq)]
```

### Storage Layout Support

Types that can be stored in contract storage implement `StorageLayout`.

```rust
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
```

### TypeInfo Support

Enables metadata generation for frontend and tooling integration.

```rust
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
```

## Usage in Contracts

### Oracle Contract Usage

```rust
use shared::{Error, Oracle};

impl shared::Oracle for Oracle {
    #[ink(message)]
    fn get_price(&self, token: AccountId) -> Option<u128> {
        self.prices.get(token)
    }
    // ... other implementations
}
```

### Registry Contract Usage

```rust
use shared::{TokenData, EnrichedTokenData, Error, Oracle};

#[ink(storage)]
pub struct Registry {
    tokens: Mapping<u32, TokenData>,
    // ... other fields
}

#[ink(message)]
pub fn get_token_data(&self, token_id: u32) -> Result<EnrichedTokenData, Error> {
    // Uses shared types throughout
}
```

## Constants and Utilities

### Polkadot Unit Constants

```rust
pub const PLANCK: u128 = 1;
pub const DOT: u128 = 10_000_000_000;      // 10^10 plancks
pub const KSM: u128 = 1_000_000_000_000;   // 10^12 plancks
```

### Basis Points Utilities

```rust
pub const MAX_WEIGHT: u32 = 10_000;  // 100% in basis points
pub const PERCENT: u32 = 100;        // 1% in basis points

/// Convert percentage to basis points
pub fn percent_to_basis_points(percent: u32) -> u32 {
    percent * PERCENT
}

/// Convert basis points to percentage
pub fn basis_points_to_percent(bp: u32) -> u32 {
    bp / PERCENT
}
```

### Tier Constants

```rust
pub const MIN_TIER: u32 = 0;  // Highest risk
pub const MAX_TIER: u32 = 5;  // Lowest risk

/// Tier descriptions
pub const TIER_DESCRIPTIONS: [&str; 6] = [
    "Experimental",    // 0
    "High Risk",       // 1
    "Medium-High Risk",// 2
    "Medium Risk",     // 3
    "Low Risk",        // 4
    "Blue Chip",       // 5
];
```

## Best Practices

### For Library Maintainers

#### **Version Compatibility**

- Use semantic versioning for breaking changes
- Maintain backward compatibility when possible
- Document breaking changes clearly

#### **Type Design**

- Keep types minimal and focused
- Use descriptive field names
- Add documentation comments

#### **Error Handling**

- Provide specific error variants
- Include helpful error messages
- Consider error recovery strategies

### For Contract Developers

#### **Import Strategy**

```rust
// Specific imports for clarity
use shared::{TokenData, Error};

// Avoid wildcard imports in production
// use shared::*;  // Don't do this
```

#### **Error Propagation**

```rust
// Proper error handling
pub fn some_function() -> Result<TokenData, Error> {
    let data = self.get_data().ok_or(Error::TokenNotFound)?;
    // Process data...
    Ok(data)
}
```

#### **Type Usage**

```rust
// Use shared types consistently
#[ink(message)]
pub fn add_token(&mut self, data: TokenData) -> Result<u32, Error> {
    // Implementation using shared types
}
```

## Testing

### Unit Tests for Shared Types

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_data_serialization() {
        let token_data = TokenData {
            token_contract: AccountId::from([0x01; 32]),
            oracle_contract: AccountId::from([0x02; 32]),
            balance: 1000,
            weight_investment: 5000,
            tier: 3,
        };

        // Test SCALE encoding/decoding
        let encoded = token_data.encode();
        let decoded = TokenData::decode(&mut &encoded[..]).unwrap();
        assert_eq!(token_data, decoded);
    }

    #[test]
    fn error_types() {
        assert_eq!(Error::Unauthorized, Error::Unauthorized);
        assert_ne!(Error::Unauthorized, Error::TokenNotFound);
    }

    #[test]
    fn basis_points_conversion() {
        assert_eq!(percent_to_basis_points(50), 5000);  // 50% = 5000 bp
        assert_eq!(basis_points_to_percent(2500), 25);  // 2500 bp = 25%
    }
}
```

### Integration Tests

```rust
// Test cross-contract type compatibility
#[test]
fn cross_contract_compatibility() {
    // Ensure types work across contract boundaries
    let oracle_response: Option<u128> = Some(10_000_000_000);
    let enriched_data = EnrichedTokenData {
        token_contract: AccountId::from([0x01; 32]),
        oracle_contract: AccountId::from([0x02; 32]),
        balance: 0,
        weight_investment: 0,
        tier: 0,
        market_cap: 0,
        market_volume: 0,
        price: oracle_response.unwrap_or(0),
    };

    assert_eq!(enriched_data.price, 10_000_000_000);
}
```

## Migration Guide

### From v0.1.0 to Future Versions

#### **Adding New Fields**

When adding fields to existing structs, consider backward compatibility:

```rust
// Before (v0.1.0)
pub struct TokenData {
    pub token_contract: AccountId,
    pub oracle_contract: AccountId,
    pub balance: u128,
    pub weight_investment: u32,
    pub tier: u32,
}

// After (v0.2.0) - Add Optional fields first
pub struct TokenData {
    pub token_contract: AccountId,
    pub oracle_contract: AccountId,
    pub balance: u128,
    pub weight_investment: u32,
    pub tier: u32,
    pub created_at: Option<u64>,      // New optional field
}

// Later (v1.0.0) - Make required if needed
pub struct TokenData {
    pub token_contract: AccountId,
    pub oracle_contract: AccountId,
    pub balance: u128,
    pub weight_investment: u32,
    pub tier: u32,
    pub created_at: u64,              // Now required
}
```

#### **Error Type Evolution**

```rust
// Add new error variants at the end
pub enum Error {
    Unauthorized,
    TokenNotFound,
    OracleCallFailed,
    InvalidParameter,
    InsufficientBalance,
    // New in v0.2.0
    ContractPaused,      // New error type
    RateLimitExceeded,   // New error type
}
```

## Extension Points

### Custom Oracle Implementations

The Oracle trait can be extended for specialized use cases:

```rust
#[ink::trait_definition]
pub trait ExtendedOracle: Oracle {
    /// Get historical price data
    #[ink(message)]
    fn get_historical_price(&self, token: AccountId, timestamp: u64) -> Option<u128>;

    /// Get price confidence interval
    #[ink(message)]
    fn get_price_confidence(&self, token: AccountId) -> Option<(u128, u128)>;
}
```

### Additional Data Types

Future versions might include:

```rust
/// Portfolio snapshot
#[derive(Decode, Encode, Clone, Debug, PartialEq)]
pub struct PortfolioSnapshot {
    pub timestamp: u64,
    pub total_value: u128,
    pub tokens: Vec<(u32, EnrichedTokenData)>,
}

/// Price alert configuration
#[derive(Decode, Encode, Clone, Debug, PartialEq)]
pub struct PriceAlert {
    pub token_id: u32,
    pub threshold: u128,
    pub alert_type: AlertType,
    pub enabled: bool,
}
```

## Troubleshooting

### Common Issues

#### **Compilation Errors**

**Issue**: `the trait bound 'TokenData: StorageLayout' is not satisfied`

```rust
// Solution: Ensure std feature is enabled for storage types
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
```

**Issue**: `cannot find type 'Error' in this scope`

```rust
// Solution: Import shared types
use shared::Error;
```

#### **Cross-Contract Call Issues**

**Issue**: Types don't match between contracts

```rust
// Solution: Ensure both contracts use same shared version
# In Cargo.toml
shared = { path = "../shared", version = "0.1.0" }
```

#### **Serialization Problems**

**Issue**: SCALE codec errors during cross-contract calls

```rust
// Solution: Ensure all types implement required traits
#[derive(Decode, Encode, Clone, Debug, PartialEq)]
```

### Debug Strategies

#### **Type Compatibility Checking**

```rust
// Add debug assertions in development
#[cfg(debug_assertions)]
fn verify_type_compatibility() {
    use shared::TokenData;

    // Verify serialization roundtrip
    let test_data = TokenData { /* ... */ };
    let encoded = test_data.encode();
    let decoded = TokenData::decode(&mut &encoded[..]).unwrap();
    assert_eq!(test_data, decoded);
}
```

#### **Error Tracing**

```rust
// Add detailed error context
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Unauthorized => write!(f, "Unauthorized: caller lacks permission"),
            Error::TokenNotFound => write!(f, "TokenNotFound: token ID does not exist"),
            // ... other variants
        }
    }
}
```

## Future Roadmap

### Planned Features

#### **v0.2.0**

- Historical data types
- Price alert structures
- Portfolio management types
- Enhanced error context

#### **v0.3.0**

- Multi-oracle support types
- Governance structures
- Staking/rewards types
- Cross-chain compatibility

#### **v1.0.0**

- Stable API guarantee
- Full documentation
- Production optimizations
- Comprehensive test suite

### Breaking Changes Policy

- **Major versions**: Breaking changes allowed
- **Minor versions**: Additive changes only
- **Patch versions**: Bug fixes only
- **Pre-1.0**: API may change significantly

## Contributing

### Code Standards

```rust
// Follow Rust naming conventions
pub struct TokenData { /* CamelCase for types */ }
pub fn get_price() { /* snake_case for functions */ }
pub const MAX_TIER: u32 = 5; /* SCREAMING_SNAKE_CASE for constants */

// Document public APIs
/// Represents token information stored in the registry
///
/// # Fields
/// - `token_contract`: The on-chain address of the token
/// - `balance`: Amount held in plancks
pub struct TokenData {
    pub token_contract: AccountId,
    pub balance: u128,
}
```

### Testing Requirements

- All public types must have serialization tests
- Error types must have equality tests
- Cross-contract compatibility tests required
- Documentation examples must compile

### Review Checklist

- [ ] All types implement required traits
- [ ] Breaking changes documented
- [ ] Tests pass
- [ ] Documentation updated
- [ ] Version bumped appropriately
