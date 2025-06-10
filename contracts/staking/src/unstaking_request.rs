// staking/src/unstaking_request.rs

use scale::{Decode, Encode};

/// Unstaking request structure
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct UnstakingRequest {
    /// Amount to unstake
    pub amount: u128,
    /// When the unstaking request was created
    pub requested_at: u64,
    /// When the tokens will be available
    pub available_at: u64,
    /// Whether the request has been claimed
    pub claimed: bool,
}