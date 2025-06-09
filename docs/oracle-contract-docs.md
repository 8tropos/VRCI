# Oracle Contract Documentation

## Overview

The Oracle contract is a price feed service that provides real-time market data for tokens. It serves as a data source for other contracts in the w3pi ecosystem, particularly the Registry contract which uses cross-contract calls to fetch live pricing information.

## Architecture

```
┌─────────────────┐
│  Oracle Contract │
├─────────────────┤
│ • Price Data    │
│ • Market Cap    │
│ • Volume Data   │
│ • Owner Access  │
└─────────────────┘
```

## Core Functionality

### **Data Storage**

- **Prices**: Token prices in plancks (Polkadot's smallest unit)
- **Market Caps**: Total market capitalization in plancks
- **Market Volumes**: 24-hour trading volume in plancks
- **Owner**: Contract owner who can update data

### **Access Control**

- Only the contract owner can update price and market data
- Read operations are public and can be called by any account or contract

## Contract Structure

### Storage

```rust
#[ink(storage)]
pub struct Oracle {
    /// Price data for tokens in plancks
    prices: Mapping<AccountId, u128>,
    /// Market cap data in plancks
    market_caps: Mapping<AccountId, u128>,
    /// Market volume data in plancks
    market_volumes: Mapping<AccountId, u128>,
    /// Contract owner
    owner: AccountId,
}
```

### Events

```rust
#[ink(event)]
pub struct PriceUpdated {
    #[ink(topic)]
    token: AccountId,
    price: u128,
}

#[ink(event)]
pub struct MarketDataUpdated {
    #[ink(topic)]
    token: AccountId,
    market_cap: u128,
    volume: u128,
}
```

## Public Interface

### Constructors

#### `new() -> Self`

Creates an empty oracle contract with the caller as owner.

**Usage:**

```bash
pop up --constructor new
```

#### `new_with_data() -> Self`

Creates an oracle with sample data pre-populated.

**Sample Data:**

- Token: `AccountId::from([0x01; 32])`
- Price: 10,000,000,000 plancks (1 DOT)
- Market Cap: 1,000,000,000,000,000 plancks (100,000 DOT)
- Volume: 100,000,000,000,000 plancks (10,000 DOT)

**Usage:**

```bash
pop up --constructor new_with_data
```

### Read Functions (Public)

#### `get_price(token: AccountId) -> Option<u128>`

Returns the current price of a token in plancks.

**Parameters:**

- `token`: The AccountId of the token contract

**Returns:**

- `Some(price)`: Price in plancks if data exists
- `None`: If no price data is available

**Example:**

```bash
pop call contract --message get_price --args TOKEN_ADDRESS
```

#### `get_market_cap(token: AccountId) -> Option<u128>`

Returns the market capitalization of a token in plancks.

**Parameters:**

- `token`: The AccountId of the token contract

**Returns:**

- `Some(market_cap)`: Market cap in plancks if data exists
- `None`: If no market cap data is available

#### `get_market_volume(token: AccountId) -> Option<u128>`

Returns the 24-hour trading volume of a token in plancks.

**Parameters:**

- `token`: The AccountId of the token contract

**Returns:**

- `Some(volume)`: Volume in plancks if data exists
- `None`: If no volume data is available

#### `get_owner() -> AccountId`

Returns the owner of the oracle contract.

### Write Functions (Owner Only)

#### `update_price(token: AccountId, price: u128) -> Result<(), Error>`

Updates the price of a specific token.

**Parameters:**

- `token`: The AccountId of the token contract
- `price`: New price in plancks

**Access Control:** Owner only

**Events Emitted:** `PriceUpdated`

**Example:**

```bash
pop call contract --message update_price --args TOKEN_ADDRESS 15000000000
```

#### `update_market_data(token: AccountId, market_cap: u128, volume: u128) -> Result<(), Error>`

Updates the market cap and volume data for a token.

**Parameters:**

- `token`: The AccountId of the token contract
- `market_cap`: Market capitalization in plancks
- `volume`: 24-hour volume in plancks

**Access Control:** Owner only

**Events Emitted:** `MarketDataUpdated`

**Example:**

```bash
pop call contract --message update_market_data --args TOKEN_ADDRESS 2000000000000000 150000000000000
```

## Polkadot Units Reference

Understanding plancks is crucial for working with the oracle:

| Unit     | Plancks           | Description             |
| -------- | ----------------- | ----------------------- |
| 1 planck | 1                 | Smallest unit           |
| 1 DOT    | 10,000,000,000    | Standard Polkadot token |
| 1 KSM    | 1,000,000,000,000 | Kusama token            |

### Example Conversions

- **$10 USD** (assuming $5/DOT): 20,000,000,000 plancks
- **100,000 DOT market cap**: 1,000,000,000,000,000 plancks
- **10,000 DOT daily volume**: 100,000,000,000,000 plancks

## Error Handling

The oracle uses the shared error types:

```rust
pub enum Error {
    Unauthorized,    // Non-owner trying to update data
    TokenNotFound,   // Token not in registry (not used in oracle)
    OracleCallFailed, // Cross-contract call failed (not used in oracle)
    InvalidParameter, // Invalid input parameter
    InsufficientBalance, // Insufficient balance (not used in oracle)
}
```

## Security Considerations

### Access Control

- **Owner Privileges**: Only the owner can update price and market data
- **Public Reads**: Anyone can read price data, enabling cross-contract calls
- **No Transfer of Ownership**: Current implementation doesn't allow changing ownership

### Data Validation

- **No Input Validation**: The oracle accepts any u128 value as valid
- **No Price Staleness**: No timestamp or expiration mechanism
- **No Data Sources**: Prices are manually set, not automatically fetched

### Potential Improvements

1. **Multi-signature ownership** for critical updates
2. **Price staleness detection** with timestamps
3. **Automatic price feeds** from external sources
4. **Price deviation limits** to prevent manipulation
5. **Emergency pause mechanism**

## Integration with Registry

The Oracle contract is designed to work with the Registry contract through cross-contract calls:

```rust
// Registry calls Oracle using CallBuilder
let price = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
    .call(oracle_address)
    .call_v1()
    .exec_input(
        ExecutionInput::new(Selector::new(ink::selector_bytes!("get_price")))
            .push_arg(token_address)
    )
    .returns::<Option<u128>>()
    .try_invoke();
```

## Testing Scenarios

### Basic Functionality

1. Deploy oracle with `new` constructor
2. Verify owner is set correctly
3. Attempt to read non-existent price (should return None)
4. Update price as owner
5. Read updated price
6. Attempt update as non-owner (should fail)

### Market Data

1. Update market data for a token
2. Verify data is stored correctly
3. Test with multiple tokens
4. Verify events are emitted

### Cross-Contract Integration

1. Deploy both Oracle and Registry
2. Add token to Registry with Oracle address
3. Call Registry's `get_token_data` to trigger cross-contract call
4. Verify Registry receives Oracle data

## Best Practices

### For Oracle Operators

1. **Regular Updates**: Keep price data current
2. **Monitoring**: Watch for unusual price movements
3. **Backup Plans**: Have multiple data sources
4. **Security**: Protect owner account private keys

### For Integrating Contracts

1. **Handle None Values**: Always check for `None` returns
2. **Gas Limits**: Set appropriate gas for cross-contract calls
3. **Error Handling**: Use `try_invoke()` for robust error handling
4. **Fallback Logic**: Have backup data sources

### For Developers

1. **Unit Tests**: Test all functions thoroughly
2. **Integration Tests**: Test cross-contract interactions
3. **Event Monitoring**: Watch for price update events
4. **Documentation**: Keep integration docs updated
