# W3PI Registry Contract Specification

## Overview
The Registry Contract serves as the central hub and coordinator for the entire W3PI ecosystem. It manages token registration, tier classification, portfolio composition, index base value tracking, and orchestrates interactions between all other contracts.

## Index Base Value System
- **Base Value**: $100 (fixed starting point for performance tracking)
- **Base Date**: Contract deployment date (recorded immutably)
- **Index Formula**: `Current Index Value = (Current Total Market Cap / Base Market Cap) × 100`
- **Performance Tracking**: Users can easily see "+25%" or "-15%" performance vs. the $100 baseline

## Updated Fee Structure
- **Buy Fee**: 0.55% (55 basis points)
- **Sell Fee**: 0.95% (95 basis points) 
- **Streaming Fee**: 1.95% annually (195 basis points)
- **Staking Fee**: 10% of rewards (handled in staking contract)

## Core Responsibilities

### 1. Token Registration & Management
**Purpose**: Maintain the official list of tokens eligible for the W3PI index

**Key Features**:
- **Token Registration**: Add new tokens with metadata (symbol, market cap, volume, unstaking periods)
- **Token Removal**: Remove underperforming tokens after grace periods
- **Grace Period Management**: 90-day waiting periods for inclusions/exclusions
- **Manual Overrides**: Emergency powers to bypass grace periods when needed
- **Token Metadata Storage**: Market cap, volume, tier classification, balances, staking amounts

**Registration Criteria**:
- Minimum market cap and volume thresholds per tier
- 90-day volume history validation
- Technical compatibility with Polkadot ecosystem
- Security audit requirements (for high-tier tokens)

### 2. Tier Classification System
**Purpose**: Automatically categorize tokens based on performance metrics

**Tier Definitions**:
- **Tier 1**: $50M market cap + $5M 90-day volume
- **Tier 2**: $250M market cap + $25M 90-day volume  
- **Tier 3**: $500M market cap + $50M 90-day volume
- **Tier 4**: $2B market cap + $200M 90-day volume
- **None**: Below minimum thresholds (grace period or removal candidate)

**80% Rule Implementation**:
- Monitor tier distribution across all registered tokens
- Automatically shift active tier when ≥80% of tokens qualify for higher tier
- Trigger portfolio rebalancing when tier changes occur
- Gradual migration to prevent market shock

### 3. Portfolio Composition Management
**Purpose**: Maintain the optimal token mix based on current tier and market conditions

**Features**:
- **Active Tier Tracking**: Monitor which tier the index currently operates in
- **Rebalance Candidate Lists**: Maintain lists of tokens eligible for rebalancing
- **Weight Calculations**: Calculate target weights based on market cap ratios
- **Portfolio Limits**: Enforce maximum position sizes and concentration limits
- **Liquidity Requirements**: Ensure minimum liquidity buffers for redemptions

### 4. Cross-Contract Coordination
**Purpose**: Orchestrate operations across the entire W3PI ecosystem

**Integration Points**:
- **Oracle Contract**: Request price feeds and market data
- **DEX Contract**: Coordinate token swaps and liquidity operations
- **Token Contract**: Manage W3PI minting, burning, and transfers
- **Staking Contract**: Coordinate staking operations and reward distribution

**Coordination Functions**:
- **Operation Sequencing**: Ensure operations happen in correct order
- **State Synchronization**: Keep all contracts in sync
- **Error Handling**: Coordinate rollbacks when operations fail
- **Event Broadcasting**: Emit events for frontend and monitoring systems

### 5. Governance & Security
**Purpose**: Provide secure, decentralized control over the registry

**Multisig Integration**:
- **3-of-5 Multisig Scheme**: Require multiple signatures for critical operations
- **Timelock Mechanisms**: 24-hour delays for sensitive changes
- **Emergency Controls**: Immediate pause capabilities for security threats
- **Backup Recovery**: Shamir's Secret Sharing for emergency access

**Permissioned Operations**:
- Token registration/removal (owners only)
- Tier adjustments (automated + owner override)
- Emergency pauses (owners + automated triggers)
- Fee adjustments (multisig required)

### 6. Data Management & Analytics
**Purpose**: Maintain historical data and provide analytics capabilities

**Snapshot System**:
- **Weekly Snapshots**: Capture prices and market caps every 7 days
- **Rolling Window**: Maintain last 4 snapshots (28-day rolling window)
- **Historical Analysis**: Support 4-week moving averages for rebalancing
- **Data Validation**: Cross-reference oracle data with external sources

**Index Value Tracking**:
- **Base Market Cap**: Immutable record of initial total market cap = $100
- **Current Index Value**: Real-time calculation using formula above
- **Performance Metrics**: Track % change from base value
- **Historical Index Values**: Maintain index value history for charts

**Performance Tracking**:
- **Portfolio Performance**: Track W3PI price evolution relative to $100 base
- **Individual Token Performance**: Monitor constituent token performance
- **Fee Collection**: Track fees collected across all operations
- **Rebalancing History**: Maintain records of all rebalancing events

### 7. Risk Management
**Purpose**: Implement safeguards to protect the index and its users

**Risk Controls**:
- **Price Deviation Limits**: Max 5% price deviation from oracle feeds
- **Emergency Pause Triggers**: Automatic pause on mass unstaking (>25% threshold)
- **Liquidity Monitoring**: Ensure sufficient liquidity for redemptions
- **Circuit Breakers**: Halt operations during extreme market conditions

**Monitoring Systems**:
- **Real-time Alerts**: Monitor for unusual activity patterns
- **Threshold Monitoring**: Track key metrics against predefined limits
- **Automated Responses**: Trigger protective measures automatically
- **Manual Overrides**: Allow emergency intervention when needed

### 9. Autonomous Zombie Stake Management
**Purpose**: Automatically handle staked positions in obsolete tokens

**Zombie Stake Detection**:
- **Monitor Tier::None Tokens**: Track tokens that have been downgraded and exceed grace period
- **Staked Balance Tracking**: Identify remaining staked amounts in obsolete tokens
- **Grace Period Validation**: Ensure 90+ days have passed since tier downgrade

**Autonomous Reallocation Process**:
- **Automatic Unstaking**: Unstake obsolete token positions during monthly rebalancing
- **Liquidation to USDC**: Convert unstaked obsolete tokens to USDC via DEX
- **Proportional Redistribution**: Reinvest proceeds into current active index tokens based on their weights
- **Event Emission**: Emit `ObsoleteStakeReallocated` events for full transparency

**Benefits**:
- **No User Action Required**: Fully automated cleanup process
- **Index Purity**: Ensures users stay exposed only to current index composition
- **Value Preservation**: Prevents stake value from being trapped in deprecated tokens
- **Transparency**: Complete audit trail of reallocation activities

## Event System
**Purpose**: Provide comprehensive logging and monitoring capabilities

**Key Events**:
- `TokenRegistered`: New token added to registry
- `TokenRemoved`: Token removed from index
- `TierShifted`: Active tier changed due to 80% rule
- `RebalanceTriggered`: Portfolio rebalancing initiated
- `EmergencyPaused`: Emergency pause activated
- `SnapshotTaken`: Weekly price/market cap snapshot created
- `IndexValueUpdated`: Base index value calculation updated
- `ObsoleteStakeReallocated`: Zombie stake automatically reallocated to active tokens

## Error Handling
**Purpose**: Graceful handling of edge cases and failures

**Error Categories**:
- **Authorization Errors**: Unauthorized access attempts
- **Data Validation Errors**: Invalid input parameters
- **State Consistency Errors**: Contract state conflicts
- **External Dependency Errors**: Oracle or DEX failures
- **Resource Limitation Errors**: Insufficient core time or funds

## Upgrade Path
**Purpose**: Enable future enhancements while maintaining stability

**Upgrade Mechanisms**:
- **Proxy Pattern**: Allow contract logic updates
- **Data Migration**: Support for moving to new contract versions
- **Backward Compatibility**: Maintain compatibility with existing integrations
- **Gradual Migration**: Phase upgrades to minimize disruption

## Integration Guidelines
**Purpose**: Standardize how other contracts interact with the registry

**Interface Standards**:
- **Function Signatures**: Standardized function names and parameters
- **Return Values**: Consistent data structures and error codes
- **Event Formats**: Standardized event schemas
- **Access Patterns**: Defined interaction flows between contracts

This registry contract serves as the "brain" of the W3PI system, coordinating all operations while maintaining security, efficiency, and regulatory compliance.