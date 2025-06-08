# Registry Contract Documentation

## Overview

The Registry contract serves as a central hub for managing token information and their associated oracle contracts. It demonstrates advanced ink! v5 features including cross-contract calls, event emission, and structured data management. The Registry acts as a bridge between token contracts and oracle data sources.

## Architecture

```
┌─────────────────┐     Cross-Contract Calls     ┌─────────────────┐
│ Registry        │────────────────────────────→│ Oracle Contract │
│ Contract        │                              │                 │
├─────────────────┤                              ├─────────────────┤
│ • Token Data    │                              │ • Price Data    │
│ • Oracle Links  │                              │ • Market Data   │
│ • Cross-calls   │                              │ • Volume Data   │
│ • Owner Access  │                              │                 │
└─────────────────┘                              └─────────────────┘
```

## Core Functionality

### **Token Management**
- Register new tokens with their associated oracle contracts
- Store token metadata (balance, weight, tier)
- Maintain unique token IDs for easy reference

### **Cross-Contract Integration**
- Fetch live price data from oracle contracts
- Combine static registry data with dynamic oracle data
- Provide enriched token information in a single call

### **Data Enrichment**
- Basic token data (stored in registry)
- Live market data (fetched from oracle)
- Combined view for comprehensive token information

## Contract Structure

### Storage

```rust
#[ink(storage)]
pub struct Registry {
    /// Mapping from token ID to token data
    tokens: Mapping<u32, TokenData>,
    /// Next available token ID
    next_token_id: u32,
    /// Registry owner
    owner: AccountId,
}
```

### Data Types

#### TokenData (Basic)
```rust
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

#### EnrichedTokenData (With Oracle Data)
```rust
pub struct EnrichedTokenData {
    pub token_contract: AccountId,
    pub oracle_contract: AccountId,
    pub balance: u128,
    pub weight_investment: u32,
    pub tier: u32,
    /// Market cap in plancks (from oracle)
    pub market_cap: u128,
    /// 24h trading volume in plancks (from oracle)
    pub market_volume: u128,
    /// Current price in plancks (from oracle)
    pub price: u128,
}
```

### Events

```rust
#[ink(event)]
pub struct TokenAdded {
    #[ink(topic)]
    token_id: u32,
    token_contract: AccountId,
    oracle_contract: AccountId,
}

#[ink(event)]
pub struct TokenUpdated {
    #[ink(topic)]
    token_id: u32,
    balance: u128,
    weight_investment: u32,
    tier: u32,
}
```

## Public Interface

### Constructor

#### `new() -> Self`
Creates an empty registry with the caller as owner.

**Usage:**
```bash
pop up --constructor new
```

### Write Functions (Owner Only)

#### `add_token(token_contract: AccountId, oracle_contract: AccountId) -> Result<u32, Error>`
Registers a new token with its associated oracle contract.

**Parameters:**
- `token_contract`: Address of the token contract
- `oracle_contract`: Address of the oracle providing price data

**Returns:**
- `Ok(token_id)`: Unique ID assigned to the token
- `Err(Error)`: If unauthorized or other error

**Access Control:** Owner only

**Events Emitted:** `TokenAdded`

**Example:**
```bash
pop call contract --message add_token --args TOKEN_ADDRESS ORACLE_ADDRESS
```

#### `update_token(token_id: u32, balance: u128, weight_investment: u32, tier: u32) -> Result<(), Error>`
Updates the registry-managed data for a token.

**Parameters:**
- `token_id`: ID of the token to update
- `balance`: New balance in plancks
- `weight_investment`: Investment weight (0-10000 basis points)
- `tier`: Token tier (0-5)

**Access Control:** Owner only

**Events Emitted:** `TokenUpdated`

**Example:**
```bash
pop call contract --message update_token --args 1 100000000000 5000 3
```

### Read Functions (Public)

#### `get_token_data(token_id: u32) -> Result<EnrichedTokenData, Error>`
**🎯 Core Function: Cross-Contract Call Magic**

Retrieves comprehensive token information by combining registry data with live oracle data.

**What Happens:**
1. Fetches token data from registry storage
2. Makes cross-contract calls to the associated oracle
3. Combines static and dynamic data into enriched response

**Cross-Contract Calls Made:**
- `oracle.get_price(token_contract)`
- `oracle.get_market_cap(token_contract)`
- `oracle.get_market_volume(token_contract)`

**Parameters:**
- `token_id`: ID of the token

**Returns:**
- `Ok(EnrichedTokenData)`: Complete token information
- `Err(Error::TokenNotFound)`: If token ID doesn't exist

**Example:**
```bash
pop call contract --message get_token_data --args 1
```

#### `get_basic_token_data(token_id: u32) -> Result<TokenData, Error>`
Retrieves only the registry-stored data without oracle calls.

**Use Case:** When you need fast access without cross-contract overhead.

#### `get_token_count() -> u32`
Returns the total number of registered tokens.

#### `token_exists(token_id: u32) -> bool`
Checks if a token ID exists in the registry.

#### `get_owner() -> AccountId`
Returns the registry owner address.

## Cross-Contract Call Implementation

The Registry uses ink! v5's CallBuilder for cross-contract calls:

```rust
let price_result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
    .call(oracle_contract_address)
    .call_v1()
    .gas_limit(0)
    .transferred_value(0)
    .exec_input(
        ink::env::call::ExecutionInput::new(
            ink::env::call::Selector::new(ink::selector_bytes!("get_price"))
        ).push_arg(token_contract)
    )
    .returns::<Option<u128>>()
    .try_invoke();
```

### Error Handling Strategy

The Registry implements robust error handling for cross-contract calls:

```rust
let price = match price_result {
    Ok(Ok(Some(p))) => p,    // Successful call with data
    _ => 0,                  // Any error returns 0
};
```

**Error Hierarchy:**
1. `try_invoke()` → `Result<Result<T, LangError>, EnvError>`
2. `Ok(Ok(value))` → Successful call
3. `Ok(Err(lang_error))` → Contract returned error
4. `Err(env_error)` → Environment/execution error

## Usage Patterns

### Basic Token Registration Workflow

1. **Deploy Oracle** with initial data
2. **Deploy Registry** 
3. **Register Token** linking it to oracle
4. **Query Token Data** to see cross-contract integration

```bash
# 1. Add token
pop call contract --message add_token --args TOKEN_ADDR ORACLE_ADDR

# 2. Get enriched data (triggers cross-contract calls)
pop call contract --message get_token_data --args 1

# 3. Update registry data
pop call contract --message update_token --args 1 500000000000 7500 4
```

### Portfolio Management Use Case

```bash
# Register multiple tokens
add_token(DOT_TOKEN, DOT_ORACLE)     # ID: 1
add_token(KSM_TOKEN, KSM_ORACLE)     # ID: 2  
add_token(CUSTOM_TOKEN, ORACLE)      # ID: 3

# Set investment weights (basis points)
update_token(1, balance, 5000, 5)    # 50% allocation, tier 5
update_token(2, balance, 3000, 4)    # 30% allocation, tier 4
update_token(3, balance, 2000, 2)    # 20% allocation, tier 2

# Get live portfolio data
get_token_data(1)  # DOT with live price
get_token_data(2)  # KSM with live price
get_token_data(3)  # Custom token with live price
```

## Security Considerations

### Access Control
- **Owner-Only Functions**: Token registration and updates
- **Public Reads**: Anyone can query token data
- **No Ownership Transfer**: Current implementation is immutable

### Cross-Contract Security
- **Oracle Trust**: Registry trusts oracle contracts completely
- **Gas Limits**: Uses unlimited gas (0) for oracle calls
- **Error Isolation**: Oracle failures don't crash registry calls

### Data Integrity
- **No Validation**: Registry doesn't validate oracle responses
- **Stale Data**: No freshness checks on oracle data
- **No Fallbacks**: Single oracle dependency per token

## Error Types

```rust
pub enum Error {
    Unauthorized,        // Non-owner attempting restricted operation
    TokenNotFound,       // Token ID doesn't exist
    OracleCallFailed,    // Cross-contract call failed (not currently used)
    InvalidParameter,    // Invalid input parameter
    InsufficientBalance, // Insufficient balance (future use)
}
```

## Performance Considerations

### Gas Usage
- **Basic Reads**: Low gas consumption
- **Cross-Contract Calls**: Higher gas due to oracle calls
- **Multiple Oracles**: Gas scales with number of oracle calls

### Optimization Strategies
1. **Batch Calls**: Group multiple token queries
2. **Caching**: Store recently fetched oracle data
3. **Selective Updates**: Only call oracle when needed
4. **Gas Limits**: Set appropriate limits for oracle calls

## Integration Examples

### Frontend Integration
```javascript
// Get token data with live prices
const tokenData = await api.query.contracts.call(
  registryAddress,
  'get_token_data',
  [tokenId]
);

console.log(`Token Price: ${tokenData.price} plancks`);
console.log(`Market Cap: ${tokenData.market_cap} plancks`);
```

### Smart Contract Integration
```rust
// Another contract calling registry
let registry: RegistryRef = registry_address.into();
let token_data = registry.get_token_data(token_id)?;
let current_price = token_data.price;
```

## Testing Scenarios

### Unit Tests
1. **Token Registration**: Add tokens and verify storage
2. **Data Updates**: Update token data and verify changes
3. **Access Control**: Test owner-only restrictions
4. **Error Handling**: Test invalid token IDs

### Integration Tests
1. **Oracle Integration**: Deploy both contracts and test cross-calls
2. **Live Data**: Verify oracle data appears in registry responses
3. **Multiple Tokens**: Test with several tokens and oracles
4. **Error Scenarios**: Test oracle failures and recovery

### End-to-End Tests
1. **Full Workflow**: Deploy, register, update, query
2. **Price Updates**: Change oracle prices and verify registry reflects changes
3. **Performance**: Test with multiple tokens under load

## Best Practices

### For Registry Operators
1. **Careful Oracle Selection**: Choose reliable oracle contracts
2. **Regular Monitoring**: Watch for failed cross-contract calls
3. **Data Validation**: Verify oracle responses make sense
4. **Backup Plans**: Have contingency for oracle failures

### For Developers
1. **Error Handling**: Always handle `TokenNotFound` errors
2. **Gas Management**: Account for cross-contract call gas costs
3. **Event Monitoring**: Watch for `TokenAdded` and `TokenUpdated` events
4. **Testing**: Test both isolated and integrated scenarios

### For Integrators
1. **Understand Data Flow**: Registry → Oracle → Response
2. **Handle Failures**: Oracle calls can fail, plan accordingly
3. **Performance**: Cross-contract calls are slower than local reads
4. **Updates**: Oracle data changes independently of registry