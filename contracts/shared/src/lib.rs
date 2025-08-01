// w3pi/contracts/shared/src/lib.rs

#![cfg_attr(not(feature = "std"), no_std)]

use ink::primitives::AccountId;
pub use scale::{Decode, Encode};

/// Token data structure shared between contracts
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

/// Enhanced token data with live oracle information
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

/// Enhanced error types for better debugging and validation
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    // Authorization errors
    Unauthorized,
    UnauthorizedRole,

    // Token management errors
    TokenNotFound,
    TokenAlreadyExists,
    InvalidTokenContract,
    ZeroAddress,

    // Parameter validation errors
    InvalidWeight, // Weight not in 0-10000 range
    InvalidTier,   // Tier not in 0-5 range
    InvalidParameter,

    // Oracle and external errors
    OracleCallFailed,

    // Business logic errors
    InsufficientBalance,
}

/// Role-based access control roles
#[derive(Debug, PartialEq, Eq, Encode, Decode, Clone, Copy)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Role {
    TokenManager,        // Can add/remove tokens
    TokenUpdater,        // Can update existing token data
    EmergencyController, // Can pause/unpause operations
}

/// Oracle trait for type-safe cross-contract calls
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
