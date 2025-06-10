// w3pi/contracts/oracle/src/lib.rs

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod oracle {
    use ink::prelude::string::String;
    use ink::storage::Mapping;
    use shared::Error;

    /// Enhanced token price data with validation metadata
    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TokenPriceData {
        /// Current price in plancks
        pub price: u128,
        /// Market cap in plancks
        pub market_cap: u128,
        /// 24h volume in plancks
        pub volume_24h: u128,
        /// Last update timestamp
        pub timestamp: u64,
    }

    /// Global validation configuration
    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct ValidationConfig {
        /// Maximum price change in basis points (e.g., 2000 = 20%)
        pub max_deviation_bp: u32,
        /// Maximum age before price is considered stale (seconds)
        pub staleness_threshold: u64,
        /// Minimum time between updates (seconds) to prevent spam
        pub min_update_interval: u64,
    }

    impl Default for ValidationConfig {
        fn default() -> Self {
            Self {
                max_deviation_bp: 2000,    // 20% max deviation
                staleness_threshold: 3600, // 1 hour staleness
                min_update_interval: 60,   // 1 minute minimum between updates
            }
        }
    }

    #[ink(storage)]
    pub struct Oracle {
        /// Enhanced price data for tokens
        token_data: Mapping<AccountId, TokenPriceData>,
        /// Authorized price updaters (in addition to owner)
        authorized_updaters: Mapping<AccountId, bool>,
        /// Global validation configuration
        validation_config: ValidationConfig,
        /// Contract owner
        owner: AccountId,
        /// Emergency pause flag
        paused: bool,
    }

    // ===== CONSTANTS =====

    /// Special address representing DOT token for USD price feeds
    const DOT_TOKEN_ADDRESS: [u8; 32] = [0xFF; 32];

    // ===== EXISTING EVENTS =====

    #[ink(event)]
    pub struct PriceUpdated {
        #[ink(topic)]
        token: AccountId,
        price: u128,
        market_cap: u128,
        volume: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct MarketDataUpdated {
        #[ink(topic)]
        token: AccountId,
        market_cap: u128,
        volume: u128,
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
    pub struct UpdaterAdded {
        #[ink(topic)]
        updater: AccountId,
    }

    #[ink(event)]
    pub struct UpdaterRemoved {
        #[ink(topic)]
        updater: AccountId,
    }

    #[ink(event)]
    pub struct EmergencyPause {
        paused: bool,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct ConfigUpdated {
        max_deviation_bp: u32,
        staleness_threshold: u64,
        min_update_interval: u64,
    }

    // ===== NEW DOT/USD EVENTS =====

    #[ink(event)]
    pub struct DotUsdPriceUpdated {
        /// USD price in scaled format (e.g., 6.50 USD = 6_500_000_000 with 9 decimals)
        usd_price: u128,
        /// Timestamp of the update
        timestamp: u64,
        /// Account that updated the price
        updated_by: AccountId,
    }

    #[ink(event)]
    pub struct DotPriceValidationFailed {
        reason: String,
        attempted_price: u128,
        current_price: u128,
        timestamp: u64,
    }

    impl Default for Oracle {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Oracle {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                token_data: Mapping::default(),
                authorized_updaters: Mapping::default(),
                validation_config: ValidationConfig::default(),
                owner: caller,
                paused: false,
            }
        }

        /// Constructor with sample data and custom config
        #[ink(constructor)]
        pub fn new_with_config(config: ValidationConfig) -> Self {
            let mut oracle = Self::new();
            oracle.validation_config = config;

            // Set default prices in plancks (1 DOT = 10^10 plancks)
            let dummy_token = AccountId::from([0x01; 32]);
            let sample_data = TokenPriceData {
                price: 10_000_000_000,             // 1 DOT
                market_cap: 1_000_000_000_000_000, // 100,000 DOT
                volume_24h: 100_000_000_000_000,   // 10,000 DOT
                timestamp: oracle.env().block_timestamp(),
            };
            oracle.token_data.insert(dummy_token, &sample_data);

            // Set initial DOT/USD price: $6.50 USD (6.5 with 9 decimal places)
            let dot_address = AccountId::from(DOT_TOKEN_ADDRESS);
            let dot_usd_data = TokenPriceData {
                price: 6_500_000_000, // $6.50 USD
                market_cap: 0,        // Not applicable for DOT price feeds
                volume_24h: 0,        // Not applicable for DOT price feeds
                timestamp: oracle.env().block_timestamp(),
            };
            oracle.token_data.insert(dot_address, &dot_usd_data);

            oracle
        }

        // ===== NEW DOT/USD PRICE MANAGEMENT =====

        /// Update DOT price in USD (for registry tier calculations)
        #[ink(message)]
        pub fn update_dot_usd_price(&mut self, usd_price: u128) -> Result<(), Error> {
            self.ensure_not_paused()?;
            self.ensure_authorized()?;

            if usd_price == 0 {
                return Err(Error::InvalidParameter);
            }

            let dot_address = AccountId::from(DOT_TOKEN_ADDRESS);
            let timestamp = self.env().block_timestamp();

            // Validate against existing DOT price if present
            if let Some(existing) = self.token_data.get(dot_address) {
                self.validate_dot_price_update(usd_price, &existing)?;
                self.validate_update_timing(&existing, timestamp)?;
            }

            let dot_price_data = TokenPriceData {
                price: usd_price, // USD price in scaled format
                market_cap: 0,    // Not applicable for DOT
                volume_24h: 0,    // Not applicable for DOT
                timestamp,
            };

            self.token_data.insert(dot_address, &dot_price_data);

            self.env().emit_event(DotUsdPriceUpdated {
                usd_price,
                timestamp,
                updated_by: self.env().caller(),
            });

            Ok(())
        }

        /// Get current DOT price in USD
        #[ink(message)]
        pub fn get_dot_usd_price(&self) -> Option<u128> {
            let dot_address = AccountId::from(DOT_TOKEN_ADDRESS);
            self.token_data.get(dot_address).map(|data| data.price)
        }

        /// Check if DOT price data is stale
        #[ink(message)]
        pub fn is_dot_price_stale(&self) -> bool {
            let dot_address = AccountId::from(DOT_TOKEN_ADDRESS);
            match self.token_data.get(dot_address) {
                Some(data) => {
                    let current_time = self.env().block_timestamp();
                    let staleness_threshold_ms = self
                        .validation_config
                        .staleness_threshold
                        .checked_mul(1000)
                        .unwrap_or(u64::MAX);

                    current_time.saturating_sub(data.timestamp) > staleness_threshold_ms
                }
                None => true, // No DOT price data is considered stale
            }
        }

        /// Get DOT price last update timestamp
        #[ink(message)]
        pub fn get_dot_price_last_update(&self) -> Option<u64> {
            let dot_address = AccountId::from(DOT_TOKEN_ADDRESS);
            self.token_data.get(dot_address).map(|data| data.timestamp)
        }

        /// Emergency DOT price override (owner only)
        #[ink(message)]
        pub fn emergency_dot_price_override(&mut self, usd_price: u128) -> Result<(), Error> {
            self.ensure_owner()?;

            if usd_price == 0 {
                return Err(Error::InvalidParameter);
            }

            let dot_address = AccountId::from(DOT_TOKEN_ADDRESS);
            let timestamp = self.env().block_timestamp();

            let dot_price_data = TokenPriceData {
                price: usd_price,
                market_cap: 0,
                volume_24h: 0,
                timestamp,
            };

            self.token_data.insert(dot_address, &dot_price_data);

            self.env().emit_event(DotUsdPriceUpdated {
                usd_price,
                timestamp,
                updated_by: self.env().caller(),
            });

            Ok(())
        }

        /// Check if given token is the DOT price token
        #[ink(message)]
        pub fn is_dot_token(&self, token: AccountId) -> bool {
            token == AccountId::from(DOT_TOKEN_ADDRESS)
        }

        /// Get the DOT token address used for USD price feeds
        #[ink(message)]
        pub fn get_dot_token_address(&self) -> AccountId {
            AccountId::from(DOT_TOKEN_ADDRESS)
        }

        // ===== CORE DATA MANAGEMENT (existing methods, unchanged) =====

        /// Update complete token data with validation
        #[ink(message)]
        pub fn update_token_data(
            &mut self,
            token: AccountId,
            price: u128,
            market_cap: u128,
            volume: u128,
        ) -> Result<(), Error> {
            self.ensure_not_paused()?;
            self.ensure_authorized()?;

            if price == 0 {
                return Err(Error::InvalidParameter);
            }

            let timestamp = self.env().block_timestamp();

            // Validate against existing data if present
            if let Some(existing) = self.token_data.get(token) {
                self.validate_price_update(token, price, &existing)?;
                self.validate_update_timing(&existing, timestamp)?;
            }

            let new_data = TokenPriceData {
                price,
                market_cap,
                volume_24h: volume,
                timestamp,
            };

            self.token_data.insert(token, &new_data);

            self.env().emit_event(PriceUpdated {
                token,
                price,
                market_cap,
                volume,
                timestamp,
            });

            Ok(())
        }

        /// Get complete token data
        #[ink(message)]
        pub fn get_token_data(&self, token: AccountId) -> Option<TokenPriceData> {
            self.token_data.get(token)
        }

        /// Get only price (backward compatibility)
        #[ink(message)]
        pub fn get_price(&self, token: AccountId) -> Option<u128> {
            self.token_data.get(token).map(|data| data.price)
        }

        /// Get market cap (backward compatibility)
        #[ink(message)]
        pub fn get_market_cap(&self, token: AccountId) -> Option<u128> {
            self.token_data.get(token).map(|data| data.market_cap)
        }

        /// Get market volume (backward compatibility)
        #[ink(message)]
        pub fn get_market_volume(&self, token: AccountId) -> Option<u128> {
            self.token_data.get(token).map(|data| data.volume_24h)
        }

        /// Check if price data is stale
        #[ink(message)]
        pub fn is_price_stale(&self, token: AccountId) -> bool {
            match self.token_data.get(token) {
                Some(data) => {
                    let current_time = self.env().block_timestamp();
                    // Fixed: Use checked multiplication to prevent overflow
                    let staleness_threshold_ms = self
                        .validation_config
                        .staleness_threshold
                        .checked_mul(1000)
                        .unwrap_or(u64::MAX); // If overflow, consider everything stale

                    current_time.saturating_sub(data.timestamp) > staleness_threshold_ms
                }
                None => true, // No data is considered stale
            }
        }

        /// Get last update timestamp
        #[ink(message)]
        pub fn get_last_update_time(&self, token: AccountId) -> Option<u64> {
            self.token_data.get(token).map(|data| data.timestamp)
        }

        // ===== AUTHORIZATION SYSTEM (unchanged) =====

        /// Add authorized updater (owner only)
        #[ink(message)]
        pub fn add_updater(&mut self, updater: AccountId) -> Result<(), Error> {
            self.ensure_owner()?;

            if updater == self.owner {
                return Err(Error::InvalidParameter); // Owner is always authorized
            }

            self.authorized_updaters.insert(updater, &true);
            self.env().emit_event(UpdaterAdded { updater });
            Ok(())
        }

        /// Remove authorized updater (owner only)
        #[ink(message)]
        pub fn remove_updater(&mut self, updater: AccountId) -> Result<(), Error> {
            self.ensure_owner()?;

            if updater == self.owner {
                return Err(Error::InvalidParameter); // Cannot remove owner
            }

            self.authorized_updaters.remove(updater);
            self.env().emit_event(UpdaterRemoved { updater });
            Ok(())
        }

        /// Check if account is authorized to update prices
        #[ink(message)]
        pub fn is_authorized_updater(&self, account: AccountId) -> bool {
            account == self.owner || self.authorized_updaters.get(account).unwrap_or(false)
        }

        // ===== CONFIGURATION MANAGEMENT (unchanged) =====

        /// Update complete validation configuration (owner only)
        #[ink(message)]
        pub fn set_validation_config(&mut self, config: ValidationConfig) -> Result<(), Error> {
            self.ensure_owner()?;

            // Basic validation of config parameters
            if config.max_deviation_bp > 10000 {
                // > 100%
                return Err(Error::InvalidParameter);
            }

            if config.staleness_threshold == 0 || config.min_update_interval == 0 {
                return Err(Error::InvalidParameter);
            }

            self.validation_config = config.clone();

            self.env().emit_event(ConfigUpdated {
                max_deviation_bp: config.max_deviation_bp,
                staleness_threshold: config.staleness_threshold,
                min_update_interval: config.min_update_interval,
            });

            Ok(())
        }

        /// Set maximum price deviation in basis points (owner only)
        /// Example: 2000 = 20% max change, 500 = 5% max change
        #[ink(message)]
        pub fn set_max_deviation(&mut self, max_deviation_bp: u32) -> Result<(), Error> {
            self.ensure_owner()?;

            if max_deviation_bp > 10000 {
                // > 100%
                return Err(Error::InvalidParameter);
            }

            self.validation_config.max_deviation_bp = max_deviation_bp;

            self.env().emit_event(ConfigUpdated {
                max_deviation_bp: self.validation_config.max_deviation_bp,
                staleness_threshold: self.validation_config.staleness_threshold,
                min_update_interval: self.validation_config.min_update_interval,
            });

            Ok(())
        }

        /// Set staleness threshold in seconds (owner only)
        /// Example: 3600 = 1 hour, 1800 = 30 minutes, 7200 = 2 hours
        #[ink(message)]
        pub fn set_staleness_threshold(&mut self, staleness_threshold: u64) -> Result<(), Error> {
            self.ensure_owner()?;

            if staleness_threshold == 0 {
                return Err(Error::InvalidParameter);
            }

            self.validation_config.staleness_threshold = staleness_threshold;

            self.env().emit_event(ConfigUpdated {
                max_deviation_bp: self.validation_config.max_deviation_bp,
                staleness_threshold: self.validation_config.staleness_threshold,
                min_update_interval: self.validation_config.min_update_interval,
            });

            Ok(())
        }

        /// Set minimum update interval in seconds (owner only)
        /// Example: 60 = 1 minute, 300 = 5 minutes, 30 = 30 seconds
        #[ink(message)]
        pub fn set_min_update_interval(&mut self, min_update_interval: u64) -> Result<(), Error> {
            self.ensure_owner()?;

            if min_update_interval == 0 {
                return Err(Error::InvalidParameter);
            }

            self.validation_config.min_update_interval = min_update_interval;

            self.env().emit_event(ConfigUpdated {
                max_deviation_bp: self.validation_config.max_deviation_bp,
                staleness_threshold: self.validation_config.staleness_threshold,
                min_update_interval: self.validation_config.min_update_interval,
            });

            Ok(())
        }

        /// Get current validation configuration
        #[ink(message)]
        pub fn get_validation_config(&self) -> ValidationConfig {
            self.validation_config.clone()
        }

        /// Get current maximum deviation in basis points
        #[ink(message)]
        pub fn get_max_deviation(&self) -> u32 {
            self.validation_config.max_deviation_bp
        }

        /// Get current staleness threshold in seconds
        #[ink(message)]
        pub fn get_staleness_threshold(&self) -> u64 {
            self.validation_config.staleness_threshold
        }

        /// Get current minimum update interval in seconds
        #[ink(message)]
        pub fn get_min_update_interval(&self) -> u64 {
            self.validation_config.min_update_interval
        }

        // ===== EMERGENCY CONTROLS (unchanged) =====

        /// Pause all price updates (owner only)
        #[ink(message)]
        pub fn pause_updates(&mut self) -> Result<(), Error> {
            self.ensure_owner()?;
            self.paused = true;

            self.env().emit_event(EmergencyPause {
                paused: true,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Resume price updates (owner only)
        #[ink(message)]
        pub fn resume_updates(&mut self) -> Result<(), Error> {
            self.ensure_owner()?;
            self.paused = false;

            self.env().emit_event(EmergencyPause {
                paused: false,
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Emergency price override (owner only)
        #[ink(message)]
        pub fn emergency_price_override(
            &mut self,
            token: AccountId,
            price: u128,
            market_cap: u128,
            volume: u128,
        ) -> Result<(), Error> {
            self.ensure_owner()?;

            if price == 0 {
                return Err(Error::InvalidParameter);
            }

            let timestamp = self.env().block_timestamp();
            let new_data = TokenPriceData {
                price,
                market_cap,
                volume_24h: volume,
                timestamp,
            };

            self.token_data.insert(token, &new_data);

            self.env().emit_event(PriceUpdated {
                token,
                price,
                market_cap,
                volume,
                timestamp,
            });

            Ok(())
        }

        /// Check if updates are paused
        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.paused
        }

        // ===== BACKWARD COMPATIBILITY (unchanged) =====

        /// Legacy update price method
        #[ink(message)]
        pub fn update_price(&mut self, token: AccountId, price: u128) -> Result<(), Error> {
            // Get existing data or use defaults
            let existing = self.token_data.get(token);
            let (market_cap, volume) = match existing {
                Some(data) => (data.market_cap, data.volume_24h),
                None => (0, 0), // New token with no market data
            };

            self.update_token_data(token, price, market_cap, volume)
        }

        /// Legacy update market data method
        #[ink(message)]
        pub fn update_market_data(
            &mut self,
            token: AccountId,
            market_cap: u128,
            volume: u128,
        ) -> Result<(), Error> {
            self.ensure_not_paused()?;
            self.ensure_authorized()?;

            let existing = self.token_data.get(token);
            match existing {
                Some(mut data) => {
                    data.market_cap = market_cap;
                    data.volume_24h = volume;
                    data.timestamp = self.env().block_timestamp();
                    self.token_data.insert(token, &data);
                }
                None => {
                    return Err(Error::InvalidParameter); // Cannot update market data without price
                }
            }

            self.env().emit_event(MarketDataUpdated {
                token,
                market_cap,
                volume,
            });

            Ok(())
        }

        /// Get the owner
        #[ink(message)]
        pub fn get_owner(&self) -> AccountId {
            self.owner
        }

        // ===== INTERNAL VALIDATION METHODS =====

        fn ensure_owner(&self) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn ensure_authorized(&self) -> Result<(), Error> {
            let caller = self.env().caller();
            if !self.is_authorized_updater(caller) {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn ensure_not_paused(&self) -> Result<(), Error> {
            if self.paused {
                return Err(Error::OracleCallFailed); // Reuse existing error
            }
            Ok(())
        }

        fn validate_price_update(
            &self,
            token: AccountId,
            new_price: u128,
            existing: &TokenPriceData,
        ) -> Result<(), Error> {
            let old_price = existing.price;

            if old_price == 0 {
                return Ok(()); // No validation against zero price
            }

            // Fixed: Use checked arithmetic for percentage change calculation
            let change_bp = if new_price > old_price {
                let price_diff = new_price.saturating_sub(old_price);
                // Use checked_mul and checked_div to prevent overflow/division errors
                match price_diff.checked_mul(10000) {
                    Some(result) => match result.checked_div(old_price) {
                        Some(change) => change,
                        None => return Err(Error::InvalidParameter), // Division error
                    },
                    None => return Err(Error::InvalidParameter), // Price change too large
                }
            } else {
                let price_diff = old_price.saturating_sub(new_price);
                // Use checked_mul and checked_div to prevent overflow/division errors
                match price_diff.checked_mul(10000) {
                    Some(result) => match result.checked_div(old_price) {
                        Some(change) => change,
                        None => return Err(Error::InvalidParameter), // Division error
                    },
                    None => return Err(Error::InvalidParameter), // Price change too large
                }
            };

            if change_bp > self.validation_config.max_deviation_bp as u128 {
                self.env().emit_event(ValidationFailed {
                    token,
                    reason: "Price deviation too high".into(),
                    attempted_price: new_price,
                    current_price: old_price,
                });
                return Err(Error::InvalidParameter);
            }

            Ok(())
        }

        /// Validate DOT price update with special handling
        fn validate_dot_price_update(
            &self,
            new_price: u128,
            existing: &TokenPriceData,
        ) -> Result<(), Error> {
            let old_price = existing.price;

            if old_price == 0 {
                return Ok(()); // No validation against zero price
            }

            // Use same validation logic as regular tokens
            let change_bp = if new_price > old_price {
                let price_diff = new_price.saturating_sub(old_price);
                match price_diff.checked_mul(10000) {
                    Some(result) => match result.checked_div(old_price) {
                        Some(change) => change,
                        None => return Err(Error::InvalidParameter),
                    },
                    None => return Err(Error::InvalidParameter),
                }
            } else {
                let price_diff = old_price.saturating_sub(new_price);
                match price_diff.checked_mul(10000) {
                    Some(result) => match result.checked_div(old_price) {
                        Some(change) => change,
                        None => return Err(Error::InvalidParameter),
                    },
                    None => return Err(Error::InvalidParameter),
                }
            };

            if change_bp > self.validation_config.max_deviation_bp as u128 {
                self.env().emit_event(DotPriceValidationFailed {
                    reason: "DOT price deviation too high".into(),
                    attempted_price: new_price,
                    current_price: old_price,
                    timestamp: self.env().block_timestamp(),
                });
                return Err(Error::InvalidParameter);
            }

            Ok(())
        }

        fn validate_update_timing(
            &self,
            existing: &TokenPriceData,
            new_timestamp: u64,
        ) -> Result<(), Error> {
            let time_diff = new_timestamp.saturating_sub(existing.timestamp);

            // Fixed: Use checked multiplication to prevent overflow
            let min_interval_ms = match self.validation_config.min_update_interval.checked_mul(1000)
            {
                Some(result) => result,
                None => return Err(Error::InvalidParameter), // Invalid configuration
            };

            if time_diff < min_interval_ms {
                return Err(Error::InvalidParameter);
            }

            Ok(())
        }
    }
}
