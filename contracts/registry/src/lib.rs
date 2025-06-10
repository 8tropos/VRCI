// w3pi/contracts/registry/src/lib.rs

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod registry {
    use ink::prelude::string::String;
    use ink::prelude::vec; // Import the vec! macro
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use shared::{EnrichedTokenData, Error, Role, TokenData};

    // ===== TIER SYSTEM DATA STRUCTURES =====

    /// Enhanced tier classification for tokens
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode, Clone, Copy, Default)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Tier {
        #[default]
        None, // Below minimum thresholds
        Tier1, // $50M market cap + $5M volume
        Tier2, // $250M market cap + $25M volume
        Tier3, // $500M market cap + $50M volume
        Tier4, // $2B market cap + $200M volume
    }

    /// Tier threshold configuration (in USD values)
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode, Clone)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TierThresholds {
        // Tier thresholds in USD (converted to plancks dynamically)
        pub tier1_market_cap_usd: u128, // $50M
        pub tier1_volume_usd: u128,     // $5M
        pub tier2_market_cap_usd: u128, // $250M
        pub tier2_volume_usd: u128,     // $25M
        pub tier3_market_cap_usd: u128, // $500M
        pub tier3_volume_usd: u128,     // $50M
        pub tier4_market_cap_usd: u128, // $2B
        pub tier4_volume_usd: u128,     // $200M
    }

    impl Default for TierThresholds {
        fn default() -> Self {
            Self {
                // Tier 1: $50M market cap, $5M volume
                tier1_market_cap_usd: 50_000_000,
                tier1_volume_usd: 5_000_000,

                // Tier 2: $250M market cap, $25M volume
                tier2_market_cap_usd: 250_000_000,
                tier2_volume_usd: 25_000_000,

                // Tier 3: $500M market cap, $50M volume
                tier3_market_cap_usd: 500_000_000,
                tier3_volume_usd: 50_000_000,

                // Tier 4: $2B market cap, $200M volume
                tier4_market_cap_usd: 2_000_000_000,
                tier4_volume_usd: 200_000_000,
            }
        }
    }

    /// Enhanced token data with tier and grace period information
    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct EnhancedTokenData {
        /// Basic token data
        pub token_contract: AccountId,
        pub oracle_contract: AccountId,
        pub balance: u128,
        pub weight_investment: u32,
        pub tier: Tier,
        /// Tier management
        pub tier_change_timestamp: Option<u64>,
        pub pending_tier_change: Option<Tier>,
    }

    impl From<TokenData> for EnhancedTokenData {
        fn from(token_data: TokenData) -> Self {
            Self {
                token_contract: token_data.token_contract,
                oracle_contract: token_data.oracle_contract,
                balance: token_data.balance,
                weight_investment: token_data.weight_investment,
                tier: Tier::None, // Will be calculated
                tier_change_timestamp: None,
                pending_tier_change: None,
            }
        }
    }

    // ===== MAIN CONTRACT STORAGE =====

    #[ink(storage)]
    pub struct Registry {
        /// Enhanced token data with tier information
        tokens: Mapping<u32, EnhancedTokenData>,
        /// Mapping from token contract to token ID (for duplicate prevention)
        token_contract_to_id: Mapping<AccountId, u32>,
        /// Role-based access control: (Role, AccountId) -> bool
        role_members: Mapping<(Role, AccountId), bool>,
        /// Next available token ID
        next_token_id: u32,
        /// Registry owner (super-admin)
        owner: AccountId,

        // ===== TIER SYSTEM STORAGE =====
        /// Current active tier for the index
        active_tier: Tier,
        /// Tier threshold configuration (in USD)
        tier_thresholds: TierThresholds,
        /// Cached tier distribution for gas optimization
        tier_distribution: Mapping<Tier, u32>,
        /// Last time active tier was changed
        last_tier_change: Option<u64>,
        /// DOT/USD oracle contract for conversion rates
        dot_usd_oracle: Option<AccountId>,

        // ===== NEW GRACE PERIOD CONFIGURATION =====
        /// Adjustable grace period in milliseconds (default: 90 days)
        grace_period_ms: u64,
    }

    // ===== ENHANCED EVENTS =====

    #[ink(event)]
    pub struct TokenAdded {
        #[ink(topic)]
        token_id: u32,
        #[ink(topic)]
        token_contract: AccountId,
        oracle_contract: AccountId,
        initial_tier: Tier,
        added_by: AccountId,
    }

    #[ink(event)]
    pub struct TokenUpdated {
        #[ink(topic)]
        token_id: u32,
        balance: u128,
        weight_investment: u32,
        old_tier: Tier,
        new_tier: Tier,
        updated_by: AccountId,
    }

    #[ink(event)]
    pub struct TokenRemoved {
        #[ink(topic)]
        token_id: u32,
        #[ink(topic)]
        token_contract: AccountId,
        tier: Tier,
        removed_by: AccountId,
    }

    #[ink(event)]
    pub struct TokenTierChanged {
        #[ink(topic)]
        token_id: u32,
        #[ink(topic)]
        token_contract: AccountId,
        old_tier: Tier,
        new_tier: Tier,
        market_cap: u128,
        volume: u128,
        reason: String, // "automatic", "manual", "grace_period_ended", "emergency_override"
    }

    #[ink(event)]
    pub struct ActiveTierShifted {
        old_tier: Tier,
        new_tier: Tier,
        trigger_reason: String, // "80_percent_rule", "manual_override"
        timestamp: u64,
        tokens_qualifying: u32,
        total_tokens: u32,
    }

    #[ink(event)]
    pub struct TierThresholdsUpdated {
        updated_by: AccountId,
        timestamp: u64,
        new_usd_rate: u128,
    }

    #[ink(event)]
    pub struct GracePeriodStarted {
        #[ink(topic)]
        token_id: u32,
        current_tier: Tier,
        pending_tier: Tier,
        grace_end_time: u64,
    }

    // ===== NEW GRACE PERIOD EVENTS =====

    #[ink(event)]
    pub struct GracePeriodUpdated {
        old_period_ms: u64,
        new_period_ms: u64,
        updated_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct EmergencyTierOverride {
        #[ink(topic)]
        token_id: u32,
        #[ink(topic)]
        token_contract: AccountId,
        old_tier: Tier,
        new_tier: Tier,
        overridden_by: AccountId,
        timestamp: u64,
        reason: String,
    }

    // ===== EXISTING EVENTS (unchanged) =====

    #[ink(event)]
    pub struct RoleGranted {
        #[ink(topic)]
        role: Role,
        #[ink(topic)]
        account: AccountId,
        granted_by: AccountId,
    }

    #[ink(event)]
    pub struct RoleRevoked {
        #[ink(topic)]
        role: Role,
        #[ink(topic)]
        account: AccountId,
        revoked_by: AccountId,
    }

    #[ink(event)]
    pub struct OperationFailed {
        operation: String,
        error: Error,
        caller: AccountId,
    }

    // ===== CONSTANTS =====

    /// Default grace period for tier changes: 90 days in milliseconds
    const DEFAULT_GRACE_PERIOD_MS: u64 = 90 * 24 * 60 * 60 * 1000; // 7,776,000,000 ms

    /// Minimum grace period: 1 hour
    const MIN_GRACE_PERIOD_MS: u64 = 60 * 60 * 1000; // 3,600,000 ms

    /// Maximum grace period: 365 days
    const MAX_GRACE_PERIOD_MS: u64 = 365 * 24 * 60 * 60 * 1000; // 31,536,000,000 ms

    /// Minimum tokens required for 80% rule calculation
    const MIN_TOKENS_FOR_TIER_SHIFT: u32 = 5;

    /// Percentage threshold for automatic tier shifting
    const TIER_SHIFT_THRESHOLD_PERCENT: u32 = 80;

    impl Default for Registry {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Registry {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            let mut registry = Self {
                tokens: Mapping::default(),
                token_contract_to_id: Mapping::default(),
                role_members: Mapping::default(),
                next_token_id: 1,
                owner: Self::env().caller(),
                active_tier: Tier::Tier1, // Start with Tier1
                tier_thresholds: TierThresholds::default(),
                tier_distribution: Mapping::default(),
                last_tier_change: None,
                dot_usd_oracle: None, // Must be set by owner after deployment
                grace_period_ms: DEFAULT_GRACE_PERIOD_MS, // 90 days default
            };

            // Initialize tier distribution cache
            registry.tier_distribution.insert(Tier::None, &0);
            registry.tier_distribution.insert(Tier::Tier1, &0);
            registry.tier_distribution.insert(Tier::Tier2, &0);
            registry.tier_distribution.insert(Tier::Tier3, &0);
            registry.tier_distribution.insert(Tier::Tier4, &0);

            registry
        }

        // ===== ROLE MANAGEMENT (unchanged) =====

        /// Grant a role to an account (owner only)
        #[ink(message)]
        pub fn grant_role(&mut self, role: Role, account: AccountId) -> Result<(), Error> {
            self.ensure_owner()?;

            if account == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            self.role_members.insert((role, account), &true);

            self.env().emit_event(RoleGranted {
                role,
                account,
                granted_by: self.env().caller(),
            });

            Ok(())
        }

        /// Revoke a role from an account (owner only)
        #[ink(message)]
        pub fn revoke_role(&mut self, role: Role, account: AccountId) -> Result<(), Error> {
            self.ensure_owner()?;

            self.role_members.remove((role, account));

            self.env().emit_event(RoleRevoked {
                role,
                account,
                revoked_by: self.env().caller(),
            });

            Ok(())
        }

        /// Check if an account has a specific role
        #[ink(message)]
        pub fn has_role(&self, role: Role, account: AccountId) -> bool {
            self.role_members.get((role, account)).unwrap_or(false)
        }

        // ===== ENHANCED TOKEN MANAGEMENT =====

        /// Add a new token to the registry with automatic tier calculation
        #[ink(message)]
        pub fn add_token(
            &mut self,
            token_contract: AccountId,
            oracle_contract: AccountId,
        ) -> Result<u32, Error> {
            self.ensure_role(Role::TokenManager)?;

            // Input validation
            if token_contract == AccountId::from([0u8; 32]) {
                self.emit_operation_failed("add_token", Error::ZeroAddress);
                return Err(Error::ZeroAddress);
            }

            if oracle_contract == AccountId::from([0u8; 32]) {
                self.emit_operation_failed("add_token", Error::ZeroAddress);
                return Err(Error::ZeroAddress);
            }

            // Check for duplicates
            if self.token_contract_to_id.contains(token_contract) {
                self.emit_operation_failed("add_token", Error::TokenAlreadyExists);
                return Err(Error::TokenAlreadyExists);
            }

            let token_id = self.next_token_id;

            // Create enhanced token data
            let mut enhanced_token_data = EnhancedTokenData {
                token_contract,
                oracle_contract,
                balance: 0,
                weight_investment: 0,
                tier: Tier::None, // Will be calculated
                tier_change_timestamp: None,
                pending_tier_change: None,
            };

            // Calculate initial tier
            let initial_tier = self
                .calculate_token_tier_internal(token_contract, oracle_contract)
                .unwrap_or(Tier::None);

            enhanced_token_data.tier = initial_tier;

            // Store token data
            self.tokens.insert(token_id, &enhanced_token_data);
            self.token_contract_to_id.insert(token_contract, &token_id);
            self.next_token_id = self.next_token_id.saturating_add(1);

            // Update tier distribution cache
            self.increment_tier_count(initial_tier);

            // Check for automatic tier shift
            self.check_and_execute_auto_tier_shift();

            self.env().emit_event(TokenAdded {
                token_id,
                token_contract,
                oracle_contract,
                initial_tier,
                added_by: self.env().caller(),
            });

            Ok(token_id)
        }

        /// Update token balance and investment data with automatic tier recalculation
        #[ink(message)]
        pub fn update_token(
            &mut self,
            token_id: u32,
            balance: u128,
            weight_investment: u32,
        ) -> Result<(), Error> {
            self.ensure_role(Role::TokenUpdater)?;

            // Parameter validation
            if weight_investment > 10000 {
                self.emit_operation_failed("update_token", Error::InvalidWeight);
                return Err(Error::InvalidWeight);
            }

            let mut token_data = self.tokens.get(token_id).ok_or_else(|| {
                self.emit_operation_failed("update_token", Error::TokenNotFound);
                Error::TokenNotFound
            })?;

            let old_tier = token_data.tier;

            // Update basic data
            token_data.balance = balance;
            token_data.weight_investment = weight_investment;

            // Recalculate tier based on current market data
            let new_tier = self
                .calculate_token_tier_internal(
                    token_data.token_contract,
                    token_data.oracle_contract,
                )
                .unwrap_or(token_data.tier);

            // Handle tier change with grace period
            if new_tier != old_tier {
                self.handle_tier_change(&mut token_data, new_tier, "automatic".into());
            }

            // Store updated data
            self.tokens.insert(token_id, &token_data);

            self.env().emit_event(TokenUpdated {
                token_id,
                balance,
                weight_investment,
                old_tier,
                new_tier: token_data.tier,
                updated_by: self.env().caller(),
            });

            Ok(())
        }

        /// Remove a token from the registry
        #[ink(message)]
        pub fn remove_token(&mut self, token_id: u32) -> Result<(), Error> {
            self.ensure_role(Role::TokenManager)?;

            let token_data = self.tokens.get(token_id).ok_or_else(|| {
                self.emit_operation_failed("remove_token", Error::TokenNotFound);
                Error::TokenNotFound
            })?;

            let token_contract = token_data.token_contract;
            let tier = token_data.tier;

            // Remove from both mappings
            self.tokens.remove(token_id);
            self.token_contract_to_id.remove(token_contract);

            // Update tier distribution cache
            self.decrement_tier_count(tier);

            // Check for automatic tier shift
            self.check_and_execute_auto_tier_shift();

            self.env().emit_event(TokenRemoved {
                token_id,
                token_contract,
                tier,
                removed_by: self.env().caller(),
            });

            Ok(())
        }

        // ===== TIER CLASSIFICATION SYSTEM =====

        /// Calculate tier for a token based on market cap and volume
        #[ink(message)]
        pub fn calculate_token_tier(&self, token_id: u32) -> Result<Tier, Error> {
            let token_data = self.tokens.get(token_id).ok_or(Error::TokenNotFound)?;

            self.calculate_token_tier_internal(
                token_data.token_contract,
                token_data.oracle_contract,
            )
            .ok_or(Error::OracleCallFailed)
        }

        /// Internal tier calculation using oracle data
        fn calculate_token_tier_internal(
            &self,
            token_contract: AccountId,
            oracle_contract: AccountId,
        ) -> Option<Tier> {
            // Get market data from oracle
            let (market_cap, volume) =
                self.get_market_data_from_oracle(token_contract, oracle_contract)?;

            // Calculate tier based on thresholds
            Some(self.calculate_tier_from_values(market_cap, volume))
        }

        /// Calculate tier based on market cap and volume values
        fn calculate_tier_from_values(&self, market_cap: u128, volume: u128) -> Tier {
            // Get DOT/USD conversion rate from oracle
            let usd_to_plancks_rate = self.get_usd_to_plancks_rate().unwrap_or({
                // Fallback: use a conservative default if oracle fails
                // 1 DOT = $5 USD (conservative estimate), 1 DOT = 10^10 plancks
                // $1 USD = 0.2 DOT = 2 × 10^9 plancks
                2_000_000_000u128
            });

            let thresholds = &self.tier_thresholds;

            // Convert USD thresholds to plancks using current conversion rate
            let tier4_market_cap_plancks = thresholds
                .tier4_market_cap_usd
                .saturating_mul(usd_to_plancks_rate);
            let tier4_volume_plancks = thresholds
                .tier4_volume_usd
                .saturating_mul(usd_to_plancks_rate);
            let tier3_market_cap_plancks = thresholds
                .tier3_market_cap_usd
                .saturating_mul(usd_to_plancks_rate);
            let tier3_volume_plancks = thresholds
                .tier3_volume_usd
                .saturating_mul(usd_to_plancks_rate);
            let tier2_market_cap_plancks = thresholds
                .tier2_market_cap_usd
                .saturating_mul(usd_to_plancks_rate);
            let tier2_volume_plancks = thresholds
                .tier2_volume_usd
                .saturating_mul(usd_to_plancks_rate);
            let tier1_market_cap_plancks = thresholds
                .tier1_market_cap_usd
                .saturating_mul(usd_to_plancks_rate);
            let tier1_volume_plancks = thresholds
                .tier1_volume_usd
                .saturating_mul(usd_to_plancks_rate);

            if market_cap >= tier4_market_cap_plancks && volume >= tier4_volume_plancks {
                Tier::Tier4
            } else if market_cap >= tier3_market_cap_plancks && volume >= tier3_volume_plancks {
                Tier::Tier3
            } else if market_cap >= tier2_market_cap_plancks && volume >= tier2_volume_plancks {
                Tier::Tier2
            } else if market_cap >= tier1_market_cap_plancks && volume >= tier1_volume_plancks {
                Tier::Tier1
            } else {
                Tier::None
            }
        }

        /// Manually update tier for a specific token (owner only)
        #[ink(message)]
        pub fn update_token_tier(&mut self, token_id: u32) -> Result<Tier, Error> {
            self.ensure_role(Role::TokenManager)?;

            let mut token_data = self.tokens.get(token_id).ok_or(Error::TokenNotFound)?;
            let old_tier = token_data.tier;

            // Calculate new tier
            let new_tier = self
                .calculate_token_tier_internal(
                    token_data.token_contract,
                    token_data.oracle_contract,
                )
                .ok_or(Error::OracleCallFailed)?;

            // Handle tier change
            if new_tier != old_tier {
                self.handle_tier_change(&mut token_data, new_tier, "manual".into());
                self.tokens.insert(token_id, &token_data);
            }

            Ok(token_data.tier)
        }

        // ===== NEW EMERGENCY OVERRIDE FUNCTIONS =====

        /// Emergency tier override - bypasses grace period (owner only)
        #[ink(message)]
        pub fn emergency_tier_override(
            &mut self,
            token_id: u32,
            new_tier: Tier,
            reason: String,
        ) -> Result<(), Error> {
            self.ensure_owner()?;

            let mut token_data = self.tokens.get(token_id).ok_or(Error::TokenNotFound)?;
            let old_tier = token_data.tier;

            if old_tier == new_tier {
                return Ok(()); // No change needed
            }

            // Update tier distribution cache
            self.decrement_tier_count(old_tier);
            self.increment_tier_count(new_tier);

            // Apply immediate tier change (bypass grace period)
            token_data.tier = new_tier;
            token_data.tier_change_timestamp = Some(self.env().block_timestamp());
            token_data.pending_tier_change = None; // Clear any pending changes

            self.tokens.insert(token_id, &token_data);

            // Emit emergency override event
            self.env().emit_event(EmergencyTierOverride {
                token_id,
                token_contract: token_data.token_contract,
                old_tier,
                new_tier,
                overridden_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
                reason: reason.clone(),
            });

            // Also emit regular tier change event for consistency
            if let Some((market_cap, volume)) = self
                .get_market_data_from_oracle(token_data.token_contract, token_data.oracle_contract)
            {
                self.env().emit_event(TokenTierChanged {
                    token_id,
                    token_contract: token_data.token_contract,
                    old_tier,
                    new_tier,
                    market_cap,
                    volume,
                    reason: "emergency_override".into(),
                });
            }

            Ok(())
        }

        /// Emergency tier override to calculated tier - bypasses grace period (owner only)
        #[ink(message)]
        pub fn emergency_tier_override_to_calculated(
            &mut self,
            token_id: u32,
            reason: String,
        ) -> Result<Tier, Error> {
            self.ensure_owner()?;

            let token_data = self.tokens.get(token_id).ok_or(Error::TokenNotFound)?;

            // Calculate what tier should be based on current market data
            let calculated_tier = self
                .calculate_token_tier_internal(
                    token_data.token_contract,
                    token_data.oracle_contract,
                )
                .ok_or(Error::OracleCallFailed)?;

            // Apply emergency override to calculated tier
            self.emergency_tier_override(token_id, calculated_tier, reason)?;

            Ok(calculated_tier)
        }

        /// Clear pending tier change (owner only)
        #[ink(message)]
        pub fn clear_pending_tier_change(&mut self, token_id: u32) -> Result<(), Error> {
            self.ensure_owner()?;

            let mut token_data = self.tokens.get(token_id).ok_or(Error::TokenNotFound)?;

            if token_data.pending_tier_change.is_some() {
                token_data.pending_tier_change = None;
                token_data.tier_change_timestamp = None;
                self.tokens.insert(token_id, &token_data);
            }

            Ok(())
        }

        // ===== NEW GRACE PERIOD MANAGEMENT =====

        /// Set grace period duration (owner only)
        #[ink(message)]
        pub fn set_grace_period(&mut self, period_ms: u64) -> Result<(), Error> {
            self.ensure_owner()?;

            // Validate grace period range
            if !(MIN_GRACE_PERIOD_MS..=MAX_GRACE_PERIOD_MS).contains(&period_ms) {
                return Err(Error::InvalidParameter);
            }

            let old_period = self.grace_period_ms;
            self.grace_period_ms = period_ms;

            self.env().emit_event(GracePeriodUpdated {
                old_period_ms: old_period,
                new_period_ms: period_ms,
                updated_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Get current grace period duration in milliseconds
        #[ink(message)]
        pub fn get_grace_period(&self) -> u64 {
            self.grace_period_ms
        }

        /// Get grace period duration in days (for convenience)
        #[ink(message)]
        pub fn get_grace_period_days(&self) -> u64 {
            self.grace_period_ms / (24 * 60 * 60 * 1000)
        }

        /// Get grace period duration in hours (for convenience)
        #[ink(message)]
        pub fn get_grace_period_hours(&self) -> u64 {
            self.grace_period_ms / (60 * 60 * 1000)
        }

        /// Get grace period limits (min/max allowed)
        #[ink(message)]
        pub fn get_grace_period_limits(&self) -> (u64, u64) {
            (MIN_GRACE_PERIOD_MS, MAX_GRACE_PERIOD_MS)
        }

        /// Calculate grace period end time for a token
        #[ink(message)]
        pub fn get_grace_period_end_time(&self, token_id: u32) -> Option<u64> {
            let token_data = self.tokens.get(token_id)?;
            let start_time = token_data.tier_change_timestamp?;
            Some(start_time.saturating_add(self.grace_period_ms))
        }

        /// Check how much time is left in grace period for a token
        #[ink(message)]
        pub fn get_grace_period_remaining(&self, token_id: u32) -> Option<u64> {
            let end_time = self.get_grace_period_end_time(token_id)?;
            let current_time = self.env().block_timestamp();

            if current_time >= end_time {
                Some(0) // Grace period expired
            } else {
                Some(end_time.saturating_sub(current_time))
            }
        }

        /// Check if grace period has expired for a token
        #[ink(message)]
        pub fn is_grace_period_expired(&self, token_id: u32) -> bool {
            match self.get_grace_period_remaining(token_id) {
                Some(remaining) => remaining == 0,
                None => false, // No grace period active
            }
        }

        // ===== EXISTING FUNCTIONS (updated to use dynamic grace period) =====

        /// Batch update tiers for all tokens (gas-intensive)
        #[ink(message)]
        pub fn refresh_all_tiers(&mut self) -> Result<u32, Error> {
            self.ensure_role(Role::TokenManager)?;

            let total_tokens = self.get_token_count();
            let mut updated_count = 0u32;

            for token_id in 1..=total_tokens {
                if let Some(mut token_data) = self.tokens.get(token_id) {
                    let old_tier = token_data.tier;

                    if let Some(new_tier) = self.calculate_token_tier_internal(
                        token_data.token_contract,
                        token_data.oracle_contract,
                    ) {
                        if new_tier != old_tier {
                            self.handle_tier_change(&mut token_data, new_tier, "scheduled".into());
                            self.tokens.insert(token_id, &token_data);
                            updated_count = updated_count.saturating_add(1);
                        }
                    }
                }
            }

            // Check for automatic tier shift after batch update
            self.check_and_execute_auto_tier_shift();

            Ok(updated_count)
        }

        /// Process tokens with expired grace periods (updated to use dynamic grace period)
        #[ink(message)]
        pub fn process_grace_periods(&mut self) -> Result<u32, Error> {
            self.ensure_role(Role::TokenUpdater)?;

            let current_time = self.env().block_timestamp();
            let mut processed_count = 0u32;

            let total_tokens = self.get_token_count();
            for token_id in 1..=total_tokens {
                if let Some(mut token_data) = self.tokens.get(token_id) {
                    if let (Some(pending_tier), Some(change_time)) = (
                        token_data.pending_tier_change,
                        token_data.tier_change_timestamp,
                    ) {
                        // Check if grace period has expired (using dynamic grace period)
                        if current_time.saturating_sub(change_time) >= self.grace_period_ms {
                            let old_tier = token_data.tier;

                            // Update tier distribution cache
                            self.decrement_tier_count(old_tier);
                            self.increment_tier_count(pending_tier);

                            // Apply the pending tier change
                            token_data.tier = pending_tier;
                            token_data.pending_tier_change = None;
                            token_data.tier_change_timestamp = Some(current_time);

                            self.tokens.insert(token_id, &token_data);
                            processed_count = processed_count.saturating_add(1);

                            // Emit tier change event
                            if let Some((market_cap, volume)) = self.get_market_data_from_oracle(
                                token_data.token_contract,
                                token_data.oracle_contract,
                            ) {
                                self.env().emit_event(TokenTierChanged {
                                    token_id,
                                    token_contract: token_data.token_contract,
                                    old_tier,
                                    new_tier: pending_tier,
                                    market_cap,
                                    volume,
                                    reason: "grace_period_ended".into(),
                                });
                            }
                        }
                    }
                }
            }

            // Check for automatic tier shift after processing grace periods
            if processed_count > 0 {
                self.check_and_execute_auto_tier_shift();
            }

            Ok(processed_count)
        }

        // ===== TIER DISTRIBUTION & 80% RULE =====

        /// Get current distribution of tokens across tiers
        #[ink(message)]
        pub fn get_tier_distribution(&self) -> Vec<(Tier, u32)> {
            let mut distribution = Vec::new();

            for tier in [
                Tier::None,
                Tier::Tier1,
                Tier::Tier2,
                Tier::Tier3,
                Tier::Tier4,
            ] {
                let count = self.tier_distribution.get(tier).unwrap_or(0);
                distribution.push((tier, count));
            }

            distribution
        }

        /// Check if 80% rule should trigger tier shift
        #[ink(message)]
        pub fn should_shift_tier(&self) -> Option<Tier> {
            let total_tokens = self.get_token_count();

            if total_tokens < MIN_TOKENS_FOR_TIER_SHIFT {
                return None;
            }

            // Check each tier higher than current active tier
            for check_tier in self.get_higher_tiers() {
                let count = self.tier_distribution.get(check_tier).unwrap_or(0);
                // Fixed: Use checked arithmetic for percentage calculation to avoid side effects
                if let Some(percentage_times_100) = count.checked_mul(100) {
                    if let Some(percentage) = percentage_times_100.checked_div(total_tokens) {
                        if percentage >= TIER_SHIFT_THRESHOLD_PERCENT {
                            return Some(check_tier);
                        }
                    }
                }
            }

            None
        }

        /// Execute tier shift (automatic or manual)
        #[ink(message)]
        pub fn shift_active_tier(&mut self, new_tier: Tier, reason: String) -> Result<(), Error> {
            // Only owner can manually override, automatic shifts are allowed
            if reason != "80_percent_rule" {
                self.ensure_owner()?;
            }

            let old_tier = self.active_tier;
            if old_tier == new_tier {
                return Ok(()); // No change needed
            }

            self.active_tier = new_tier;
            self.last_tier_change = Some(self.env().block_timestamp());

            let total_tokens = self.get_token_count();
            let qualifying_tokens = self.tier_distribution.get(new_tier).unwrap_or(0);

            self.env().emit_event(ActiveTierShifted {
                old_tier,
                new_tier,
                trigger_reason: reason,
                timestamp: self.env().block_timestamp(),
                tokens_qualifying: qualifying_tokens,
                total_tokens,
            });

            Ok(())
        }

        /// Automatic tier shift check and execution
        fn check_and_execute_auto_tier_shift(&mut self) {
            if let Some(new_tier) = self.should_shift_tier() {
                let _ = self.shift_active_tier(new_tier, "80_percent_rule".into());
            }
        }

        // ===== TIER CONFIGURATION MANAGEMENT =====

        /// Set DOT/USD oracle contract (owner only)
        #[ink(message)]
        pub fn set_dot_usd_oracle(&mut self, oracle_contract: AccountId) -> Result<(), Error> {
            self.ensure_owner()?;

            if oracle_contract == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            self.dot_usd_oracle = Some(oracle_contract);
            Ok(())
        }

        /// Get current DOT/USD oracle contract
        #[ink(message)]
        pub fn get_dot_usd_oracle(&self) -> Option<AccountId> {
            self.dot_usd_oracle
        }

        /// Update tier thresholds in USD (owner only)
        #[ink(message)]
        pub fn set_tier_thresholds(&mut self, thresholds: TierThresholds) -> Result<(), Error> {
            self.ensure_owner()?;

            // Basic validation - ensure thresholds are in ascending order
            if thresholds.tier1_market_cap_usd >= thresholds.tier2_market_cap_usd
                || thresholds.tier2_market_cap_usd >= thresholds.tier3_market_cap_usd
                || thresholds.tier3_market_cap_usd >= thresholds.tier4_market_cap_usd
            {
                return Err(Error::InvalidParameter);
            }

            if thresholds.tier1_volume_usd >= thresholds.tier2_volume_usd
                || thresholds.tier2_volume_usd >= thresholds.tier3_volume_usd
                || thresholds.tier3_volume_usd >= thresholds.tier4_volume_usd
            {
                return Err(Error::InvalidParameter);
            }

            self.tier_thresholds = thresholds;

            self.env().emit_event(TierThresholdsUpdated {
                updated_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
                new_usd_rate: self.get_usd_to_plancks_rate().unwrap_or(0),
            });

            Ok(())
        }

        /// Get current tier thresholds
        #[ink(message)]
        pub fn get_tier_thresholds(&self) -> TierThresholds {
            self.tier_thresholds.clone()
        }

        /// Get current active tier
        #[ink(message)]
        pub fn get_active_tier(&self) -> Tier {
            self.active_tier
        }

        /// Get last tier change timestamp
        #[ink(message)]
        pub fn get_last_tier_change(&self) -> Option<u64> {
            self.last_tier_change
        }

        /// Get current USD to plancks conversion rate from oracle
        #[ink(message)]
        pub fn get_current_usd_rate(&self) -> Option<u128> {
            self.get_usd_to_plancks_rate()
        }

        // ===== ENHANCED QUERY FUNCTIONS =====

        /// Get enhanced token data with tier information
        #[ink(message)]
        pub fn get_enhanced_token_data(&self, token_id: u32) -> Result<EnhancedTokenData, Error> {
            self.tokens.get(token_id).ok_or(Error::TokenNotFound)
        }

        /// Get token data with live oracle prices (backward compatibility)
        #[ink(message)]
        pub fn get_token_data(&self, token_id: u32) -> Result<EnrichedTokenData, Error> {
            let token_data = self.tokens.get(token_id).ok_or(Error::TokenNotFound)?;

            // Use CallBuilder for cross-contract calls to deployed oracle
            let price_result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call(token_data.oracle_contract)
                .call_v1()
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("get_price"),
                    ))
                    .push_arg(token_data.token_contract),
                )
                .returns::<Option<u128>>()
                .try_invoke();

            let market_cap_result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call(token_data.oracle_contract)
                .call_v1()
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("get_market_cap"),
                    ))
                    .push_arg(token_data.token_contract),
                )
                .returns::<Option<u128>>()
                .try_invoke();

            let market_volume_result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call(token_data.oracle_contract)
                .call_v1()
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("get_market_volume"),
                    ))
                    .push_arg(token_data.token_contract),
                )
                .returns::<Option<u128>>()
                .try_invoke();

            // Extract values with proper error handling
            let price = match price_result {
                Ok(Ok(Some(p))) => p,
                _ => 0,
            };

            let market_cap = match market_cap_result {
                Ok(Ok(Some(mc))) => mc,
                _ => 0,
            };

            let market_volume = match market_volume_result {
                Ok(Ok(Some(mv))) => mv,
                _ => 0,
            };

            let enriched_data = EnrichedTokenData {
                token_contract: token_data.token_contract,
                oracle_contract: token_data.oracle_contract,
                balance: token_data.balance,
                weight_investment: token_data.weight_investment,
                tier: match token_data.tier {
                    Tier::None => 0,
                    Tier::Tier1 => 1,
                    Tier::Tier2 => 2,
                    Tier::Tier3 => 3,
                    Tier::Tier4 => 4,
                },
                market_cap,
                market_volume,
                price,
            };

            Ok(enriched_data)
        }

        /// Get tokens by tier
        #[ink(message)]
        pub fn get_tokens_by_tier(&self, tier: Tier) -> Vec<u32> {
            let mut tokens = Vec::new();
            let total_tokens = self.get_token_count();

            for token_id in 1..=total_tokens {
                if let Some(token_data) = self.tokens.get(token_id) {
                    if token_data.tier == tier {
                        tokens.push(token_id);
                    }
                }
            }

            tokens
        }

        /// Get tokens with pending tier changes
        #[ink(message)]
        pub fn get_tokens_with_pending_changes(&self) -> Vec<(u32, Tier, Tier, u64)> {
            let mut pending_tokens = Vec::new();
            let total_tokens = self.get_token_count();

            for token_id in 1..=total_tokens {
                if let Some(token_data) = self.tokens.get(token_id) {
                    if let (Some(pending_tier), Some(change_time)) = (
                        token_data.pending_tier_change,
                        token_data.tier_change_timestamp,
                    ) {
                        pending_tokens.push((token_id, token_data.tier, pending_tier, change_time));
                    }
                }
            }

            pending_tokens
        }

        // ===== EXISTING QUERY FUNCTIONS (updated) =====

        /// Get total number of registered tokens
        #[ink(message)]
        pub fn get_token_count(&self) -> u32 {
            self.next_token_id.saturating_sub(1)
        }

        /// Get the owner
        #[ink(message)]
        pub fn get_owner(&self) -> AccountId {
            self.owner
        }

        /// Check if a token exists
        #[ink(message)]
        pub fn token_exists(&self, token_id: u32) -> bool {
            self.tokens.contains(token_id)
        }

        /// Get basic token data without oracle calls (backward compatibility)
        #[ink(message)]
        pub fn get_basic_token_data(&self, token_id: u32) -> Result<TokenData, Error> {
            let enhanced_data = self.tokens.get(token_id).ok_or(Error::TokenNotFound)?;

            Ok(TokenData {
                token_contract: enhanced_data.token_contract,
                oracle_contract: enhanced_data.oracle_contract,
                balance: enhanced_data.balance,
                weight_investment: enhanced_data.weight_investment,
                tier: match enhanced_data.tier {
                    Tier::None => 0,
                    Tier::Tier1 => 1,
                    Tier::Tier2 => 2,
                    Tier::Tier3 => 3,
                    Tier::Tier4 => 4,
                },
            })
        }

        /// Get token ID by token contract address
        #[ink(message)]
        pub fn get_token_id_by_contract(&self, token_contract: AccountId) -> Option<u32> {
            self.token_contract_to_id.get(token_contract)
        }

        // ===== INTERNAL HELPER FUNCTIONS =====

        /// Handle tier change with grace period logic (updated to use dynamic grace period)
        fn handle_tier_change(
            &mut self,
            token_data: &mut EnhancedTokenData,
            new_tier: Tier,
            reason: String,
        ) {
            let old_tier = token_data.tier;
            let current_time = self.env().block_timestamp();

            // For immediate changes (manual override or emergency), skip grace period
            if reason == "manual_override" || reason == "emergency" {
                // Update tier distribution cache
                self.decrement_tier_count(old_tier);
                self.increment_tier_count(new_tier);

                token_data.tier = new_tier;
                token_data.tier_change_timestamp = Some(current_time);
                token_data.pending_tier_change = None;

                // Emit tier change event
                if let Some((market_cap, volume)) = self.get_market_data_from_oracle(
                    token_data.token_contract,
                    token_data.oracle_contract,
                ) {
                    self.env().emit_event(TokenTierChanged {
                        token_id: self
                            .token_contract_to_id
                            .get(token_data.token_contract)
                            .unwrap_or(0),
                        token_contract: token_data.token_contract,
                        old_tier,
                        new_tier,
                        market_cap,
                        volume,
                        reason,
                    });
                }
            } else {
                // Start grace period for automatic changes (using dynamic grace period)
                token_data.pending_tier_change = Some(new_tier);
                token_data.tier_change_timestamp = Some(current_time);

                let grace_end_time = current_time.saturating_add(self.grace_period_ms);

                self.env().emit_event(GracePeriodStarted {
                    token_id: self
                        .token_contract_to_id
                        .get(token_data.token_contract)
                        .unwrap_or(0),
                    current_tier: old_tier,
                    pending_tier: new_tier,
                    grace_end_time,
                });
            }
        }

        /// Get market data from oracle (helper function)
        fn get_market_data_from_oracle(
            &self,
            token_contract: AccountId,
            oracle_contract: AccountId,
        ) -> Option<(u128, u128)> {
            // Get market cap
            let market_cap_result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call(oracle_contract)
                .call_v1()
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("get_market_cap"),
                    ))
                    .push_arg(token_contract),
                )
                .returns::<Option<u128>>()
                .try_invoke();

            // Get market volume
            let market_volume_result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call(oracle_contract)
                .call_v1()
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("get_market_volume"),
                    ))
                    .push_arg(token_contract),
                )
                .returns::<Option<u128>>()
                .try_invoke();

            // Extract values
            let market_cap = match market_cap_result {
                Ok(Ok(Some(mc))) => mc,
                _ => return None,
            };

            let volume = match market_volume_result {
                Ok(Ok(Some(mv))) => mv,
                _ => return None,
            };

            Some((market_cap, volume))
        }

        /// Get tiers higher than current active tier
        fn get_higher_tiers(&self) -> Vec<Tier> {
            match self.active_tier {
                Tier::None => vec![Tier::Tier1, Tier::Tier2, Tier::Tier3, Tier::Tier4],
                Tier::Tier1 => vec![Tier::Tier2, Tier::Tier3, Tier::Tier4],
                Tier::Tier2 => vec![Tier::Tier3, Tier::Tier4],
                Tier::Tier3 => vec![Tier::Tier4],
                Tier::Tier4 => vec![], // Already at highest tier
            }
        }

        /// Increment tier count in distribution cache
        fn increment_tier_count(&mut self, tier: Tier) {
            let current_count = self.tier_distribution.get(tier).unwrap_or(0);
            self.tier_distribution
                .insert(tier, &(current_count.saturating_add(1)));
        }

        /// Decrement tier count in distribution cache
        fn decrement_tier_count(&mut self, tier: Tier) {
            let current_count = self.tier_distribution.get(tier).unwrap_or(0);
            if current_count > 0 {
                self.tier_distribution
                    .insert(tier, &(current_count.saturating_sub(1)));
            }
        }

        /// Ensure caller is owner
        fn ensure_owner(&self) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        /// Ensure caller has required role (or is owner)
        fn ensure_role(&self, role: Role) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller == self.owner || self.has_role(role, caller) {
                Ok(())
            } else {
                Err(Error::UnauthorizedRole)
            }
        }

        /// Get USD to plancks conversion rate from DOT/USD oracle
        fn get_usd_to_plancks_rate(&self) -> Option<u128> {
            let oracle_contract = self.dot_usd_oracle?;

            // Get DOT price in USD from oracle (assuming DOT is represented by a special address)
            let dot_token_address = AccountId::from([0xFF; 32]); // Special address for DOT itself

            let dot_price_result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call(oracle_contract)
                .call_v1()
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("get_price"),
                    ))
                    .push_arg(dot_token_address),
                )
                .returns::<Option<u128>>()
                .try_invoke();

            match dot_price_result {
                Ok(Ok(Some(dot_price_in_usd_plancks))) => {
                    // dot_price_in_usd_plancks represents how many plancks 1 DOT is worth in USD
                    // We need: how many plancks = $1 USD
                    // If 1 DOT = $6 USD (6 * 10^10 plancks in USD terms)
                    // Then $1 USD = (10^10 / 6) plancks = 1.67 * 10^9 plancks

                    // Assuming the oracle returns USD price in plancks (scaled appropriately)
                    // We need to convert this to "plancks per USD"
                    let one_dot_in_plancks = 10_000_000_000u128; // 1 DOT = 10^10 plancks

                    // Fixed: Use checked arithmetic to prevent side effects
                    if dot_price_in_usd_plancks > 0 {
                        // USD to plancks rate = (plancks per DOT) / (USD per DOT)
                        one_dot_in_plancks.checked_div(dot_price_in_usd_plancks)
                    } else {
                        None
                    }
                }
                _ => None, // Oracle call failed
            }
        }

        /// Emit operation failed event for monitoring
        fn emit_operation_failed(&self, operation: &str, error: Error) {
            self.env().emit_event(OperationFailed {
                operation: String::from(operation),
                error,
                caller: self.env().caller(),
            });
        }
    }
}
