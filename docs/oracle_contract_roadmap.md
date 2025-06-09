# Enhanced Oracle Contract - Functions & Roadmap

## Overview

Registry-focused oracle contract combining price validation, historical tracking, and multi-source data management for portfolio management use cases.

## Core Functions Design

### **Data Storage & Types**

```rust
pub struct TokenPriceData {
    pub price: u128,              // Current price in plancks
    pub market_cap: u128,         // Market cap in plancks
    pub volume_24h: u128,         // 24h volume in plancks
    pub timestamp: u64,           // Last update timestamp
    pub source_count: u8,         // Number of sources used
    pub confidence: u8,           // Price confidence score (0-100)
    pub twap: u128,              // Time-weighted average price
}

pub struct ValidationConfig {
    pub max_deviation: u32,       // Max price change % (basis points)
    pub staleness_threshold: u64, // Max age before stale (seconds)
    pub min_sources: u8,          // Minimum sources for consensus
    pub update_frequency: u64,    // Target update interval (seconds)
}
```

### **Phase 1: Core Functionality (Week 1-2)**

#### **1.1 Basic Data Management**

```rust
// Core data operations
#[ink(message)]
pub fn update_token_data(&mut self, token: AccountId, price: u128, market_cap: u128, volume: u128) -> Result<(), Error>

#[ink(message)]
pub fn get_token_data(&self, token: AccountId) -> Option<TokenPriceData>

#[ink(message)]
pub fn get_price(&self, token: AccountId) -> Option<u128>

#[ink(message)]
pub fn is_price_stale(&self, token: AccountId) -> bool
```

#### **1.2 Authorization System**

```rust
// Multi-tier authorization
#[ink(message)]
pub fn add_updater(&mut self, updater: AccountId) -> Result<(), Error>

#[ink(message)]
pub fn remove_updater(&mut self, updater: AccountId) -> Result<(), Error>

#[ink(message)]
pub fn is_authorized_updater(&self, account: AccountId) -> bool
```

#### **1.3 Configuration Management**

```rust
// Owner-configurable parameters
#[ink(message)]
pub fn set_validation_config(&mut self, token: AccountId, config: ValidationConfig) -> Result<(), Error>

#[ink(message)]
pub fn set_global_config(&mut self, config: ValidationConfig) -> Result<(), Error>

#[ink(message)]
pub fn get_config(&self, token: AccountId) -> ValidationConfig
```

### **Phase 2: Validation & Safety (Week 3)**

#### **2.1 Price Validation**

```rust
// Data quality controls
#[ink(message)]
pub fn submit_price_with_validation(&mut self, token: AccountId, price: u128, source_id: u8) -> Result<(), Error>

// Internal validation logic
fn validate_price_deviation(&self, token: AccountId, new_price: u128) -> Result<(), Error>
fn validate_timestamp(&self, timestamp: u64) -> Result<(), Error>
fn validate_source_consensus(&self, token: AccountId) -> Result<(), Error>
```

#### **2.2 Emergency Controls**

```rust
// Circuit breakers
#[ink(message)]
pub fn pause_updates(&mut self) -> Result<(), Error>

#[ink(message)]
pub fn emergency_price_override(&mut self, token: AccountId, price: u128) -> Result<(), Error>

#[ink(message)]
pub fn is_paused(&self) -> bool
```

### **Phase 3: Historical Data & TWAP (Week 4)**

#### **3.1 Historical Tracking**

```rust
// Price history management
#[ink(message)]
pub fn get_price_history(&self, token: AccountId, duration: u64) -> Vec<(u64, u128)>

#[ink(message)]
pub fn calculate_twap(&self, token: AccountId, window: u64) -> Option<u128>

// Internal history management
fn add_to_price_history(&mut self, token: AccountId, price: u128, timestamp: u64)
fn cleanup_old_history(&mut self, token: AccountId)
```

#### **3.2 Analytics Functions**

```rust
// Portfolio-focused analytics
#[ink(message)]
pub fn get_price_change_24h(&self, token: AccountId) -> Option<i32> // Basis points

#[ink(message)]
pub fn get_volatility(&self, token: AccountId, window: u64) -> Option<u32>

#[ink(message)]
pub fn get_last_update_time(&self, token: AccountId) -> Option<u64>
```

### **Phase 4: Economic Layer (Week 5)**

#### **4.1 Gas Management**

```rust
// Economic sustainability
#[ink(message)]
pub fn deposit_gas_funds(&mut self) -> Result<(), Error>

#[ink(message)]
pub fn withdraw_gas_funds(&mut self, amount: u128) -> Result<(), Error>

#[ink(message)]
pub fn refund_gas_to_updater(&mut self, updater: AccountId, amount: u128) -> Result<(), Error>

#[ink(message)]
pub fn get_gas_pool_balance(&self) -> u128
```

#### **4.2 Fee Collection (Optional)**

```rust
// Revenue from oracle consumers
#[ink(message)]
pub fn set_usage_fee(&mut self, fee: u128) -> Result<(), Error>

#[ink(message)]
pub fn collect_usage_fee(&mut self) -> Result<(), Error>
```

### **Phase 5: Registry Integration (Week 6)**

#### **5.1 Batch Operations**

```rust
// Efficient multi-token updates
#[ink(message)]
pub fn batch_update_prices(&mut self, updates: Vec<(AccountId, u128, u128, u128)>) -> Result<(), Error>

#[ink(message)]
pub fn batch_get_token_data(&self, tokens: Vec<AccountId>) -> Vec<Option<TokenPriceData>>
```

#### **5.2 Registry-Specific Helpers**

```rust
// Portfolio calculation helpers
#[ink(message)]
pub fn get_portfolio_value(&self, tokens: Vec<(AccountId, u128)>) -> Option<u128> // (token, balance) -> total_value

#[ink(message)]
pub fn get_price_feed_health(&self) -> (u32, u32) // (healthy_feeds, total_feeds)
```

### **Phase 6: Advanced Features (Week 7-8)**

#### **6.1 Source Management**

```rust
// Multi-source tracking
#[ink(message)]
pub fn register_price_source(&mut self, source_id: u8, name: String) -> Result<(), Error>

#[ink(message)]
pub fn submit_source_price(&mut self, token: AccountId, price: u128, source_id: u8) -> Result<(), Error>

#[ink(message)]
pub fn get_source_reliability(&self, source_id: u8) -> Option<u8> // 0-100 reliability score
```

#### **6.2 Advanced Analytics**

```rust
// Sophisticated portfolio metrics
#[ink(message)]
pub fn get_correlation(&self, token_a: AccountId, token_b: AccountId, window: u64) -> Option<i32>

#[ink(message)]
pub fn get_portfolio_risk_metrics(&self, tokens: Vec<AccountId>) -> Option<RiskMetrics>
```

## Events System

```rust
#[ink(event)]
pub struct PriceUpdated {
    #[ink(topic)]
    token: AccountId,
    price: u128,
    timestamp: u64,
    source_count: u8,
    confidence: u8,
}

#[ink(event)]
pub struct ValidationFailed {
    #[ink(topic)]
    token: AccountId,
    reason: String,
    attempted_price: u128,
    current_price: u128,
}

#[ink(event)]
pub struct EmergencyPause {
    reason: String,
    timestamp: u64,
}

#[ink(event)]
pub struct ConfigUpdated {
    #[ink(topic)]
    token: AccountId,
    parameter: String,
    old_value: u128,
    new_value: u128,
}
```

## Error Handling

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    // Authorization
    NotAuthorized,
    NotOwner,

    // Validation
    PriceDeviationTooHigh,
    StalePrice,
    InsufficientSources,
    InvalidTimestamp,
    InvalidPrice,

    // System
    ContractPaused,
    InsufficientGasFunds,
    HistoryBufferFull,

    // Configuration
    InvalidConfig,
    ConfigNotFound,
}
```

## Testing Strategy

### **Unit Tests (Each Phase)**

- Function input validation
- Authorization checks
- Mathematical calculations (TWAP, deviations)
- Edge cases (zero prices, overflow conditions)

### **Integration Tests**

- Registry contract integration
- Multi-source consensus scenarios
- Emergency pause/resume workflows
- Gas refund mechanisms

### **Performance Tests**

- Batch operation efficiency
- Historical data retrieval speed
- Memory usage with large price histories
- Gas optimization validation

## Deployment Roadmap

### **Week 1-2: MVP Deployment**

- Basic price storage and retrieval
- Simple authorization
- Registry integration

### **Week 3-4: Production Ready**

- Full validation system
- TWAP calculations
- Historical tracking

### **Week 5-6: Economic Layer**

- Gas management
- Batch operations
- Advanced registry features

### **Week 7-8: Advanced Features**

- Multi-source management
- Portfolio analytics
- Performance optimization

## Success Metrics

- **Data Quality**: <1% failed validations, <5min average staleness
- **Performance**: Batch updates <500ms, single queries <100ms
- **Reliability**: 99.9% uptime, <0.1% transaction failures
- **Economics**: Gas costs <10% of update value, sustainable operation
