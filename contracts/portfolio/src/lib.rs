// w3pi/contracts/portfolio/src/lib.rs

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod portfolio {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use shared::Error; // Assuming we'll use shared error types
    use ink::prelude::format;
    // ===== CORE DATA TYPES =====

    /// Portfolio state for emergency controls
    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum PortfolioState {
        Active,      // Normal operations
        Paused,      // Emergency pause - no trades
        Maintenance, // Rebalancing in progress
        Emergency,   // Emergency state - withdrawals only
    }

    impl Default for PortfolioState {
        fn default() -> Self {
            Self::Active
        }
    }

    /// Fee configuration structure
    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct FeeConfiguration {
        /// Buy fee in basis points (default: 55 = 0.55%)
        pub buy_fee_bp: u32,
        /// Sell fee in basis points (default: 95 = 0.95%)
        pub sell_fee_bp: u32,
        /// Streaming fee in basis points annually (default: 195 = 1.95%)
        pub streaming_fee_bp: u32,
    }

    impl Default for FeeConfiguration {
        fn default() -> Self {
            Self {
                buy_fee_bp: 55,        // 0.55%
                sell_fee_bp: 95,       // 0.95%
                streaming_fee_bp: 195, // 1.95% annually
            }
        }
    }

    /// Holdings data for a specific token
    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq, Default)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TokenHolding {
        /// Amount of tokens held
        pub amount: u128,
        /// Target weight in basis points (0-10000)
        pub target_weight_bp: u32,
        /// Last rebalance timestamp
        pub last_rebalance: u64,
        /// Accumulated fees from this token
        pub fees_collected: u128,
    }

    /// Portfolio composition summary
    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioComposition {
        pub total_tokens: u32,
        pub total_value: u128,
        pub holdings: Vec<(u32, TokenHolding)>, // (token_id, holding_data)
    }

    /// Enhanced token data from Registry (local copy for type compatibility)
    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct EnrichedTokenData {
        pub token_contract: AccountId,
        pub oracle_contract: AccountId,
        pub balance: u128,
        pub weight_investment: u32,
        pub tier: u32,
        pub market_cap: u128,
        pub market_volume: u128,
        pub price: u128,
    }

    // ===== MAIN CONTRACT STORAGE =====

    #[ink(storage)]
    pub struct Portfolio {
        // ===== BASIC CONTRACT MANAGEMENT =====
        /// Contract owner (has admin privileges)
        owner: AccountId,
        /// Current portfolio state
        state: PortfolioState,
        /// Contract deployment timestamp
        deployment_timestamp: u64,

        // ===== HOLDINGS MANAGEMENT =====
        /// Token holdings: token_id -> holding data
        holdings: Mapping<u32, TokenHolding>,
        /// List of token IDs we hold (for iteration)
        held_token_ids: Vec<u32>,
        /// Total number of unique tokens held
        total_tokens_held: u32,

        // ===== INDEX BASE VALUE SYSTEM =====
        /// Fixed base value: $100 in plancks (immutable)
        index_base_value: u128,
        /// Total portfolio value at deployment (immutable baseline)
        base_portfolio_value: u128,
        /// Current calculated index value in plancks
        current_index_value: u128,
        /// Last time index value was updated
        last_index_update: u64,
        /// Index calculation enabled flag
        index_tracking_enabled: bool,

        // ===== FEE SYSTEM =====
        /// Fee configuration
        fee_config: FeeConfiguration,
        /// Total fees collected per token: token_id -> fee_amount
        collected_fees: Mapping<u32, u128>,
        /// Last streaming fee collection timestamp per user: user -> timestamp
        last_streaming_fee: Mapping<AccountId, u64>,
        /// Fee beneficiary addresses and their share: beneficiary -> share_bp
        fee_beneficiaries: Mapping<AccountId, u32>,
        /// Total collected fees in USDC equivalent
        total_fees_collected: u128,

        // ===== EXTERNAL CONTRACT REFERENCES =====
        /// Registry contract for token metadata and tiers
        registry_contract: Option<AccountId>,
        /// W3PI token contract for minting/burning
        token_contract: Option<AccountId>,
        /// DEX contract for token swaps
        dex_contract: Option<AccountId>,
        /// Oracle contract for price feeds (usually accessed via Registry)
        oracle_contract: Option<AccountId>,

        // ===== PORTFOLIO MANAGEMENT =====
        /// Maximum number of tokens portfolio can hold
        max_tokens: u32,
        /// Minimum portfolio value before allowing trades
        min_portfolio_value: u128,
        /// Last rebalancing timestamp
        last_rebalance: u64,
        /// Rebalancing threshold in basis points (e.g., 500 = 5% deviation triggers rebalance)
        rebalance_threshold_bp: u32,
        /// Emergency pause flag for all operations
        emergency_paused: bool,

        // ===== LIQUIDITY & RISK MANAGEMENT =====
        /// Minimum USDC buffer for liquidity
        min_liquidity_buffer: u128,
        /// Current USDC holdings for liquidity
        usdc_balance: u128,
        /// Maximum single token position as % of portfolio (in basis points)
        max_single_position_bp: u32,
        /// Slippage tolerance for trades (in basis points)
        max_slippage_bp: u32,
    }

    // ===== EVENTS FRAMEWORK =====

    // Portfolio State Events
    #[ink(event)]
    pub struct PortfolioStateChanged {
        old_state: PortfolioState,
        new_state: PortfolioState,
        changed_by: AccountId,
        timestamp: u64,
        reason: String,
    }

    #[ink(event)]
    pub struct PortfolioInitialized {
        owner: AccountId,
        deployment_timestamp: u64,
        index_base_value: u128,
        initial_state: PortfolioState,
    }

    // Holdings Events
    #[ink(event)]
    pub struct TokenHoldingAdded {
        #[ink(topic)]
        token_id: u32,
        amount: u128,
        target_weight_bp: u32,
        added_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct TokenHoldingUpdated {
        #[ink(topic)]
        token_id: u32,
        old_amount: u128,
        new_amount: u128,
        old_weight: u32,
        new_weight: u32,
        updated_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct TokenHoldingRemoved {
        #[ink(topic)]
        token_id: u32,
        final_amount: u128,
        removed_by: AccountId,
        timestamp: u64,
    }

    // Index Base Value Events
    #[ink(event)]
    pub struct IndexValueUpdated {
        old_value: u128,
        new_value: u128,
        performance_bp: i32, // Performance in basis points vs base
        total_portfolio_value: u128,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct BasePortfolioValueSet {
        base_value: u128,
        base_portfolio_value: u128,
        deployment_timestamp: u64,
        set_by: AccountId,
    }

    // Configuration Events
    #[ink(event)]
    pub struct FeeConfigurationUpdated {
        old_config: FeeConfiguration,
        new_config: FeeConfiguration,
        updated_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct ContractReferenceUpdated {
        contract_type: String, // "registry", "token", "dex", "oracle"
        old_address: Option<AccountId>,
        new_address: AccountId,
        updated_by: AccountId,
        timestamp: u64,
    }

    #[ink(event)]
    pub struct EmergencyPause {
        paused: bool,
        paused_by: AccountId,
        timestamp: u64,
        reason: String,
    }

    #[ink(event)]
    pub struct OperationFailed {
        operation: String,
        error: String,
        caller: AccountId,
        timestamp: u64,
    }

    // ===== CONSTANTS =====

    /// Default maximum tokens portfolio can hold
    const DEFAULT_MAX_TOKENS: u32 = 50;

    /// Default rebalancing threshold (5%)
    const DEFAULT_REBALANCE_THRESHOLD_BP: u32 = 500;

    /// Default maximum single position (20% of portfolio)
    const DEFAULT_MAX_SINGLE_POSITION_BP: u32 = 2000;

    /// Default maximum slippage tolerance (2%)
    const DEFAULT_MAX_SLIPPAGE_BP: u32 = 200;

    /// Index base value: $100 in plancks (assuming 1 DOT = 10^10 plancks)
    const INDEX_BASE_VALUE: u128 = 100_000_000_000; // $100

    /// Minimum portfolio value: $1000 in plancks
    const MIN_PORTFOLIO_VALUE: u128 = 1_000_000_000_000; // $1000

    /// Default minimum liquidity buffer: $100 in USDC
    const DEFAULT_MIN_LIQUIDITY_BUFFER: u128 = 100_000_000_000; // $100

    // ===== IMPLEMENTATION =====

    impl Default for Portfolio {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Portfolio {
        /// Constructor - Initialize empty portfolio
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let timestamp = Self::env().block_timestamp();

            let portfolio = Self {
                // Basic contract management
                owner: caller,
                state: PortfolioState::Active,
                deployment_timestamp: timestamp,

                // Holdings management
                holdings: Mapping::default(),
                held_token_ids: Vec::new(),
                total_tokens_held: 0,

                // Index base value system
                index_base_value: INDEX_BASE_VALUE,
                base_portfolio_value: 0, // Will be set when first tokens are added
                current_index_value: INDEX_BASE_VALUE,
                last_index_update: timestamp,
                index_tracking_enabled: false, // Enable after initialization

                // Fee system
                fee_config: FeeConfiguration::default(),
                collected_fees: Mapping::default(),
                last_streaming_fee: Mapping::default(),
                fee_beneficiaries: Mapping::default(),
                total_fees_collected: 0,

                // External contract references
                registry_contract: None,
                token_contract: None,
                dex_contract: None,
                oracle_contract: None,

                // Portfolio management
                max_tokens: DEFAULT_MAX_TOKENS,
                min_portfolio_value: MIN_PORTFOLIO_VALUE,
                last_rebalance: timestamp,
                rebalance_threshold_bp: DEFAULT_REBALANCE_THRESHOLD_BP,
                emergency_paused: false,

                // Liquidity & risk management
                min_liquidity_buffer: DEFAULT_MIN_LIQUIDITY_BUFFER,
                usdc_balance: 0,
                max_single_position_bp: DEFAULT_MAX_SINGLE_POSITION_BP,
                max_slippage_bp: DEFAULT_MAX_SLIPPAGE_BP,
            };

            Self::env().emit_event(PortfolioInitialized {
                owner: caller,
                deployment_timestamp: timestamp,
                index_base_value: INDEX_BASE_VALUE,
                initial_state: PortfolioState::Active,
            });

            portfolio
        }

        // ===== BASIC ACCESS CONTROL =====

        /// Ensure caller is the owner
        fn ensure_owner(&self) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        /// Ensure portfolio is in active state
        #[allow(dead_code)]
        fn ensure_active(&self) -> Result<(), Error> {
            match self.state {
                PortfolioState::Active => Ok(()),
                _ => Err(Error::InvalidParameter), // Portfolio not active
            }
        }

        /// Ensure portfolio is not emergency paused
        fn ensure_not_emergency_paused(&self) -> Result<(), Error> {
            if self.emergency_paused {
                return Err(Error::InvalidParameter); // Emergency paused
            }
            Ok(())
        }

        // ===== BASIC GETTERS =====

        /// Get portfolio owner
        #[ink(message)]
        pub fn get_owner(&self) -> AccountId {
            self.owner
        }

        /// Get current portfolio state
        #[ink(message)]
        pub fn get_state(&self) -> PortfolioState {
            self.state.clone()
        }

        /// Get deployment timestamp
        #[ink(message)]
        pub fn get_deployment_timestamp(&self) -> u64 {
            self.deployment_timestamp
        }

        /// Get total number of tokens held
        #[ink(message)]
        pub fn get_total_tokens_held(&self) -> u32 {
            self.total_tokens_held
        }

        /// Get list of token IDs currently held
        #[ink(message)]
        pub fn get_held_token_ids(&self) -> Vec<u32> {
            self.held_token_ids.clone()
        }

        /// Check if portfolio is emergency paused
        #[ink(message)]
        pub fn is_emergency_paused(&self) -> bool {
            self.emergency_paused
        }

        /// Get fee configuration
        #[ink(message)]
        pub fn get_fee_config(&self) -> FeeConfiguration {
            self.fee_config.clone()
        }

        /// Get total fees collected
        #[ink(message)]
        pub fn get_total_fees_collected(&self) -> u128 {
            self.total_fees_collected
        }

        // ===== BASIC SETTERS (OWNER ONLY) =====

        /// Set portfolio state (owner only)
        #[ink(message)]
        pub fn set_state(
            &mut self,
            new_state: PortfolioState,
            reason: String,
        ) -> Result<(), Error> {
            self.ensure_owner()?;

            let old_state = self.state.clone();
            self.state = new_state.clone();

            self.env().emit_event(PortfolioStateChanged {
                old_state,
                new_state,
                changed_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
                reason,
            });

            Ok(())
        }

        /// Emergency pause all operations (owner only)
        #[ink(message)]
        pub fn emergency_pause(&mut self, reason: String) -> Result<(), Error> {
            self.ensure_owner()?;

            self.emergency_paused = true;
            self.state = PortfolioState::Emergency;

            self.env().emit_event(EmergencyPause {
                paused: true,
                paused_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
                reason,
            });

            Ok(())
        }

        /// Resume operations after emergency pause (owner only)
        #[ink(message)]
        pub fn resume_operations(&mut self, reason: String) -> Result<(), Error> {
            self.ensure_owner()?;

            self.emergency_paused = false;
            self.state = PortfolioState::Active;

            self.env().emit_event(EmergencyPause {
                paused: false,
                paused_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
                reason,
            });

            Ok(())
        }

        /// Update fee configuration (owner only)
        #[ink(message)]
        pub fn set_fee_config(&mut self, new_config: FeeConfiguration) -> Result<(), Error> {
            self.ensure_owner()?;

            // Validate fee configuration
            if new_config.buy_fee_bp > 10000
                || new_config.sell_fee_bp > 10000
                || new_config.streaming_fee_bp > 10000
            {
                return Err(Error::InvalidParameter);
            }

            let old_config = self.fee_config.clone();
            self.fee_config = new_config.clone();

            self.env().emit_event(FeeConfigurationUpdated {
                old_config,
                new_config,
                updated_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        // ===== CONTRACT REFERENCE MANAGEMENT =====

        /// Set registry contract address (owner only)
        #[ink(message)]
        pub fn set_registry_contract(&mut self, registry: AccountId) -> Result<(), Error> {
            self.ensure_owner()?;

            let old_address = self.registry_contract;
            self.registry_contract = Some(registry);

            self.env().emit_event(ContractReferenceUpdated {
                contract_type: String::from("registry"),
                old_address,
                new_address: registry,
                updated_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Set token contract address (owner only)
        #[ink(message)]
        pub fn set_token_contract(&mut self, token: AccountId) -> Result<(), Error> {
            self.ensure_owner()?;

            let old_address = self.token_contract;
            self.token_contract = Some(token);

            self.env().emit_event(ContractReferenceUpdated {
                contract_type: String::from("token"),
                old_address,
                new_address: token,
                updated_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Set DEX contract address (owner only)
        #[ink(message)]
        pub fn set_dex_contract(&mut self, dex: AccountId) -> Result<(), Error> {
            self.ensure_owner()?;

            let old_address = self.dex_contract;
            self.dex_contract = Some(dex);

            self.env().emit_event(ContractReferenceUpdated {
                contract_type: String::from("dex"),
                old_address,
                new_address: dex,
                updated_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Set oracle contract address (owner only)
        #[ink(message)]
        pub fn set_oracle_contract(&mut self, oracle: AccountId) -> Result<(), Error> {
            self.ensure_owner()?;

            let old_address = self.oracle_contract;
            self.oracle_contract = Some(oracle);

            self.env().emit_event(ContractReferenceUpdated {
                contract_type: String::from("oracle"),
                old_address,
                new_address: oracle,
                updated_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Get contract references
        #[ink(message)]
        pub fn get_registry_contract(&self) -> Option<AccountId> {
            self.registry_contract
        }

        #[ink(message)]
        pub fn get_token_contract(&self) -> Option<AccountId> {
            self.token_contract
        }

        #[ink(message)]
        pub fn get_dex_contract(&self) -> Option<AccountId> {
            self.dex_contract
        }

        #[ink(message)]
        pub fn get_oracle_contract(&self) -> Option<AccountId> {
            self.oracle_contract
        }

        // ===== PHASE 2: HOLDINGS MANAGEMENT =====

        /// Add a new token holding to the portfolio (owner only)
        #[ink(message)]
        pub fn add_token_holding(
            &mut self,
            token_id: u32,
            amount: u128,
            target_weight_bp: u32,
        ) -> Result<(), Error> {
            self.ensure_owner()?;
            self.ensure_not_emergency_paused()?;

            // Validate inputs
            if amount == 0 {
                self.emit_operation_failed("add_token_holding", "Amount cannot be zero");
                return Err(Error::InvalidParameter);
            }

            if target_weight_bp > 10000 {
                self.emit_operation_failed("add_token_holding", "Target weight cannot exceed 100%");
                return Err(Error::InvalidParameter);
            }

            // Check if we already hold this token
            if self.holdings.contains(token_id) {
                self.emit_operation_failed("add_token_holding", "Token already held");
                return Err(Error::TokenAlreadyExists);
            }

            // Check maximum tokens limit
            if self.total_tokens_held >= self.max_tokens {
                self.emit_operation_failed("add_token_holding", "Maximum tokens limit reached");
                return Err(Error::InvalidParameter);
            }

            // Check total weight allocation doesn't exceed 100%
            let current_weight = self.calculate_total_target_weight();
            let total_weight = current_weight.saturating_add(target_weight_bp);
            if total_weight > 10000 {
                self.emit_operation_failed(
                    "add_token_holding",
                    "Total target weight would exceed 100%",
                );
                return Err(Error::InvalidParameter);
            }

            let timestamp = self.env().block_timestamp();

            // Create new token holding
            let holding = TokenHolding {
                amount,
                target_weight_bp,
                last_rebalance: timestamp,
                fees_collected: 0,
            };

            // Store the holding
            self.holdings.insert(token_id, &holding);
            self.held_token_ids.push(token_id);
            self.total_tokens_held = self.total_tokens_held.saturating_add(1);

            // Trigger index update
            self.trigger_index_update();

            // Emit event
            self.env().emit_event(TokenHoldingAdded {
                token_id,
                amount,
                target_weight_bp,
                added_by: self.env().caller(),
                timestamp,
            });

            Ok(())
        }

        /// Update an existing token holding (owner only)
        #[ink(message)]
        pub fn update_token_holding(
            &mut self,
            token_id: u32,
            new_amount: u128,
            new_target_weight_bp: u32,
        ) -> Result<(), Error> {
            self.ensure_owner()?;
            self.ensure_not_emergency_paused()?;

            // Validate target weight
            if new_target_weight_bp > 10000 {
                self.emit_operation_failed(
                    "update_token_holding",
                    "Target weight cannot exceed 100%",
                );
                return Err(Error::InvalidParameter);
            }

            // Get existing holding
            let mut holding = self.holdings.get(token_id).ok_or_else(|| {
                self.emit_operation_failed("update_token_holding", "Token not found");
                Error::TokenNotFound
            })?;

            // Check total weight allocation
            let current_total_weight = self.calculate_total_target_weight();

            // Use checked arithmetic for weight change calculation
            let weight_change = if new_target_weight_bp >= holding.target_weight_bp {
                new_target_weight_bp.saturating_sub(holding.target_weight_bp)
            } else {
                holding
                    .target_weight_bp
                    .saturating_sub(new_target_weight_bp)
            };

            let new_total_weight = if new_target_weight_bp >= holding.target_weight_bp {
                current_total_weight.saturating_add(weight_change)
            } else {
                current_total_weight.saturating_sub(weight_change)
            };

            if new_total_weight > 10000 {
                self.emit_operation_failed(
                    "update_token_holding",
                    "Total target weight would exceed 100%",
                );
                return Err(Error::InvalidParameter);
            }

            // Store old values for event
            let old_amount = holding.amount;
            let old_weight = holding.target_weight_bp;

            // Update holding
            holding.amount = new_amount;
            holding.target_weight_bp = new_target_weight_bp;
            holding.last_rebalance = self.env().block_timestamp();

            // Store updated holding
            self.holdings.insert(token_id, &holding);

            // Trigger index update
            self.trigger_index_update();

            // Emit event
            self.env().emit_event(TokenHoldingUpdated {
                token_id,
                old_amount,
                new_amount,
                old_weight,
                new_weight: new_target_weight_bp,
                updated_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Remove a token holding from the portfolio (owner only)
        #[ink(message)]
        pub fn remove_token_holding(&mut self, token_id: u32) -> Result<(), Error> {
            self.ensure_owner()?;
            self.ensure_not_emergency_paused()?;

            // Get existing holding
            let holding = self.holdings.get(token_id).ok_or_else(|| {
                self.emit_operation_failed("remove_token_holding", "Token not found");
                Error::TokenNotFound
            })?;

            let final_amount = holding.amount;

            // Remove from storage
            self.holdings.remove(token_id);

            // Remove from token IDs list
            if let Some(pos) = self.held_token_ids.iter().position(|&x| x == token_id) {
                self.held_token_ids.remove(pos);
            }

            self.total_tokens_held = self.total_tokens_held.saturating_sub(1);

            // Trigger index update
            self.trigger_index_update();

            // Emit event
            self.env().emit_event(TokenHoldingRemoved {
                token_id,
                final_amount,
                removed_by: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Get specific token holding data
        #[ink(message)]
        pub fn get_token_holding(&self, token_id: u32) -> Option<TokenHolding> {
            self.holdings.get(token_id)
        }

        /// Check if portfolio holds a specific token
        #[ink(message)]
        pub fn holds_token(&self, token_id: u32) -> bool {
            self.holdings.contains(token_id)
        }

        /// Get complete portfolio composition
        #[ink(message)]
        pub fn get_portfolio_composition(&self) -> PortfolioComposition {
            let mut holdings_vec = Vec::new();
            let mut total_value = 0u128;

            // Collect all holdings
            for token_id in &self.held_token_ids {
                if let Some(holding) = self.holdings.get(*token_id) {
                    // For now, use amount as value (will be replaced with actual value calculation in later phases)
                    total_value = total_value.saturating_add(holding.amount);
                    holdings_vec.push((*token_id, holding));
                }
            }

            PortfolioComposition {
                total_tokens: self.total_tokens_held,
                total_value,
                holdings: holdings_vec,
            }
        }

        /// Get token holding amount only (convenience method)
        #[ink(message)]
        pub fn get_token_amount(&self, token_id: u32) -> u128 {
            self.holdings.get(token_id).map(|h| h.amount).unwrap_or(0)
        }

        /// Get token target weight only (convenience method)
        #[ink(message)]
        pub fn get_token_target_weight(&self, token_id: u32) -> u32 {
            self.holdings
                .get(token_id)
                .map(|h| h.target_weight_bp)
                .unwrap_or(0)
        }

        /// Get all holdings as simple (token_id, amount) pairs
        #[ink(message)]
        pub fn get_all_holdings(&self) -> Vec<(u32, u128)> {
            let mut holdings_vec = Vec::new();

            for token_id in &self.held_token_ids {
                if let Some(holding) = self.holdings.get(*token_id) {
                    holdings_vec.push((*token_id, holding.amount));
                }
            }

            holdings_vec
        }

        /// Get all target weights as (token_id, weight_bp) pairs
        #[ink(message)]
        pub fn get_all_target_weights(&self) -> Vec<(u32, u32)> {
            let mut weights_vec = Vec::new();

            for token_id in &self.held_token_ids {
                if let Some(holding) = self.holdings.get(*token_id) {
                    weights_vec.push((*token_id, holding.target_weight_bp));
                }
            }

            weights_vec
        }

        /// Calculate total target weight allocation across all holdings
        #[ink(message)]
        pub fn get_total_target_weight(&self) -> u32 {
            self.calculate_total_target_weight()
        }

        /// Check if portfolio has any holdings
        #[ink(message)]
        pub fn has_holdings(&self) -> bool {
            self.total_tokens_held > 0
        }

        /// Get portfolio statistics
        #[ink(message)]
        pub fn get_portfolio_stats(&self) -> (u32, u32, u32) {
            // Returns: (total_tokens_held, total_target_weight_bp, max_tokens_allowed)
            (
                self.total_tokens_held,
                self.calculate_total_target_weight(),
                self.max_tokens,
            )
        }

        // ===== BATCH OPERATIONS =====

        /// Add multiple token holdings in a single transaction (owner only)
        #[ink(message)]
        pub fn add_multiple_holdings(
            &mut self,
            holdings_data: Vec<(u32, u128, u32)>, // (token_id, amount, target_weight_bp)
        ) -> Result<u32, Error> {
            self.ensure_owner()?;
            self.ensure_not_emergency_paused()?;

            if holdings_data.is_empty() {
                return Err(Error::InvalidParameter);
            }

            // Validate total operation won't exceed limits
            let new_token_count = holdings_data.len() as u32;
            if self.total_tokens_held.saturating_add(new_token_count) > self.max_tokens {
                self.emit_operation_failed(
                    "add_multiple_holdings",
                    "Would exceed maximum tokens limit",
                );
                return Err(Error::InvalidParameter);
            }

            // Calculate total weight for validation
            let mut total_new_weight = 0u32;
            for (token_id, amount, target_weight_bp) in &holdings_data {
                // Validate each input
                if *amount == 0 || *target_weight_bp > 10000 {
                    self.emit_operation_failed("add_multiple_holdings", "Invalid amount or weight");
                    return Err(Error::InvalidParameter);
                }

                // Check for duplicates in input
                if self.holdings.contains(*token_id) {
                    self.emit_operation_failed(
                        "add_multiple_holdings",
                        "Duplicate token in portfolio",
                    );
                    return Err(Error::TokenAlreadyExists);
                }

                total_new_weight = total_new_weight.saturating_add(*target_weight_bp);
            }

            // Check total weight allocation
            let current_total_weight = self.calculate_total_target_weight();
            if current_total_weight.saturating_add(total_new_weight) > 10000 {
                self.emit_operation_failed(
                    "add_multiple_holdings",
                    "Total weight would exceed 100%",
                );
                return Err(Error::InvalidParameter);
            }

            let timestamp = self.env().block_timestamp();
            let mut added_count = 0u32;

            // Add all holdings
            for (token_id, amount, target_weight_bp) in holdings_data {
                let holding = TokenHolding {
                    amount,
                    target_weight_bp,
                    last_rebalance: timestamp,
                    fees_collected: 0,
                };

                self.holdings.insert(token_id, &holding);
                self.held_token_ids.push(token_id);
                self.total_tokens_held = self.total_tokens_held.saturating_add(1);
                added_count = added_count.saturating_add(1);

                // Emit event for each token
                self.env().emit_event(TokenHoldingAdded {
                    token_id,
                    amount,
                    target_weight_bp,
                    added_by: self.env().caller(),
                    timestamp,
                });
            }

            Ok(added_count)
        }

        /// Update multiple token amounts in a single transaction (owner only)
        #[ink(message)]
        pub fn update_multiple_amounts(
            &mut self,
            updates: Vec<(u32, u128)>, // (token_id, new_amount)
        ) -> Result<u32, Error> {
            self.ensure_owner()?;
            self.ensure_not_emergency_paused()?;

            if updates.is_empty() {
                return Err(Error::InvalidParameter);
            }

            let timestamp = self.env().block_timestamp();
            let mut updated_count = 0u32;

            for (token_id, new_amount) in updates {
                if let Some(mut holding) = self.holdings.get(token_id) {
                    let old_amount = holding.amount;
                    holding.amount = new_amount;
                    holding.last_rebalance = timestamp;

                    self.holdings.insert(token_id, &holding);
                    updated_count = updated_count.saturating_add(1);

                    // Emit event
                    self.env().emit_event(TokenHoldingUpdated {
                        token_id,
                        old_amount,
                        new_amount,
                        old_weight: holding.target_weight_bp,
                        new_weight: holding.target_weight_bp, // Weight unchanged
                        updated_by: self.env().caller(),
                        timestamp,
                    });
                }
            }

            Ok(updated_count)
        }

        // ===== VALIDATION & LIMITS MANAGEMENT =====

        /// Set maximum tokens limit (owner only)
        #[ink(message)]
        pub fn set_max_tokens(&mut self, max_tokens: u32) -> Result<(), Error> {
            self.ensure_owner()?;

            if max_tokens == 0 || max_tokens < self.total_tokens_held {
                return Err(Error::InvalidParameter);
            }

            self.max_tokens = max_tokens;
            Ok(())
        }

        /// Get maximum tokens limit
        #[ink(message)]
        pub fn get_max_tokens(&self) -> u32 {
            self.max_tokens
        }

        /// Check if portfolio can accept more tokens
        #[ink(message)]
        pub fn can_add_tokens(&self, count: u32) -> bool {
            self.total_tokens_held.saturating_add(count) <= self.max_tokens
        }

        /// Validate portfolio weight allocation is correct
        #[ink(message)]
        pub fn validate_weight_allocation(&self) -> Result<bool, Error> {
            let total_weight = self.calculate_total_target_weight();
            Ok(total_weight <= 10000)
        }

        /// Get remaining weight allocation capacity
        #[ink(message)]
        pub fn get_remaining_weight_capacity(&self) -> u32 {
            let total_weight = self.calculate_total_target_weight();
            if total_weight < 10000 {
                10000_u32.saturating_sub(total_weight)
            } else {
                0
            }
        }

        // ===== INTERNAL HELPER METHODS =====

        /// Calculate total target weight across all holdings
        fn calculate_total_target_weight(&self) -> u32 {
            let mut total_weight = 0u32;

            for token_id in &self.held_token_ids {
                if let Some(holding) = self.holdings.get(*token_id) {
                    total_weight = total_weight.saturating_add(holding.target_weight_bp);
                }
            }

            total_weight
        }

        // ===== PHASE 3: INDEX BASE VALUE SYSTEM =====

        /// Initialize base portfolio value after first tokens are added (owner only)
        /// This sets the immutable baseline for performance tracking
        #[ink(message)]
        pub fn initialize_base_portfolio_value(&mut self) -> Result<(), Error> {
            self.ensure_owner()?;

            // Can only initialize once
            if self.base_portfolio_value != 0 {
                self.emit_operation_failed(
                    "initialize_base_portfolio_value",
                    "Base value already initialized",
                );
                return Err(Error::InvalidParameter);
            }

            // Must have some holdings to initialize
            if self.total_tokens_held == 0 {
                self.emit_operation_failed(
                    "initialize_base_portfolio_value",
                    "No holdings to calculate base value",
                );
                return Err(Error::InvalidParameter);
            }

            // Calculate current portfolio value as baseline
            let total_value = self.calculate_total_portfolio_value()?;

            if total_value == 0 {
                self.emit_operation_failed(
                    "initialize_base_portfolio_value",
                    "Portfolio value is zero",
                );
                return Err(Error::InvalidParameter);
            }

            // Set immutable baseline values
            self.base_portfolio_value = total_value;
            self.current_index_value = self.index_base_value; // Start at $100
            self.index_tracking_enabled = true;
            self.last_index_update = self.env().block_timestamp();

            // Emit initialization event
            self.env().emit_event(BasePortfolioValueSet {
                base_value: self.index_base_value,
                base_portfolio_value: total_value,
                deployment_timestamp: self.deployment_timestamp,
                set_by: self.env().caller(),
            });

            Ok(())
        }

        /// Calculate current index value using the formula:
        /// Index Value = (Current Portfolio Value / Base Portfolio Value) × Base Index Value
        #[ink(message)]
        pub fn calculate_current_index_value(&self) -> Result<u128, Error> {
            if !self.index_tracking_enabled || self.base_portfolio_value == 0 {
                return Ok(self.index_base_value); // Return base value if not initialized
            }

            let current_portfolio_value = self.calculate_total_portfolio_value()?;

            // Prevent division by zero
            if self.base_portfolio_value == 0 {
                return Ok(self.index_base_value);
            }

            // Calculate: (current_value / base_value) × base_index_value
            // Use checked arithmetic to prevent overflow
            let index_value = current_portfolio_value
                .checked_mul(self.index_base_value)
                .ok_or(Error::InvalidParameter)?
                .checked_div(self.base_portfolio_value)
                .ok_or(Error::InvalidParameter)?;

            Ok(index_value)
        }

        /// Get current cached index value (fast query)
        #[ink(message)]
        pub fn get_current_index_value(&self) -> u128 {
            self.current_index_value
        }

        /// Update cached index value with real-time calculation (owner only)
        #[ink(message)]
        pub fn update_index_value(&mut self) -> Result<u128, Error> {
            self.ensure_owner()?;

            if !self.index_tracking_enabled {
                return Ok(self.index_base_value);
            }

            let old_value = self.current_index_value;
            let new_value = self.calculate_current_index_value()?;

            self.current_index_value = new_value;
            self.last_index_update = self.env().block_timestamp();

            // Calculate performance in basis points
            let performance_bp = self.calculate_performance_bp(new_value)?;

            // Get current portfolio value for event
            let total_portfolio_value = self.calculate_total_portfolio_value().unwrap_or(0);

            // Emit update event
            self.env().emit_event(IndexValueUpdated {
                old_value,
                new_value,
                performance_bp,
                total_portfolio_value,
                timestamp: self.env().block_timestamp(),
            });

            Ok(new_value)
        }

        /// Get index performance as basis points relative to $100 baseline
        /// Returns: +2500 for +25%, -1500 for -15%, etc.
        #[ink(message)]
        pub fn get_index_performance(&self) -> Result<i32, Error> {
            self.calculate_performance_bp(self.current_index_value)
        }

        /// Get real-time index performance (recalculates current value)
        #[ink(message)]
        pub fn get_realtime_index_performance(&self) -> Result<i32, Error> {
            let current_value = self.calculate_current_index_value()?;
            self.calculate_performance_bp(current_value)
        }

        /// Get index base metrics for UI display
        #[ink(message)]
        pub fn get_index_base_metrics(&self) -> (u128, u128, u64, bool) {
            (
                self.index_base_value,       // $100 baseline
                self.base_portfolio_value,   // Portfolio value at initialization
                self.deployment_timestamp,   // When contract was deployed
                self.index_tracking_enabled, // Whether tracking is active
            )
        }

        /// Get index value in USD (converted via DOT/USD oracle)
        #[ink(message)]
        pub fn get_index_value_usd(&self) -> Result<u128, Error> {
            // Get current index value in plancks
            let index_value_plancks = self.current_index_value;

            // Convert to USD using oracle rate
            self.convert_plancks_to_usd(index_value_plancks)
        }

        /// Get real-time index value in USD
        #[ink(message)]
        pub fn get_realtime_index_value_usd(&self) -> Result<u128, Error> {
            let current_value = self.calculate_current_index_value()?;
            self.convert_plancks_to_usd(current_value)
        }

        /// Check if index value data is stale (not updated recently)
        #[ink(message)]
        pub fn is_index_value_stale(&self) -> bool {
            if !self.index_tracking_enabled {
                return false; // Not tracking, so not stale
            }

            let current_time = self.env().block_timestamp();
            let staleness_threshold = 3_600_000u64; // 1 hour in milliseconds

            current_time.saturating_sub(self.last_index_update) > staleness_threshold
        }

        /// Get time since last index update in milliseconds
        #[ink(message)]
        pub fn get_index_update_age(&self) -> u64 {
            let current_time = self.env().block_timestamp();
            current_time.saturating_sub(self.last_index_update)
        }

        /// Force index recalculation and update (owner only)
        #[ink(message)]
        pub fn refresh_index_value(&mut self) -> Result<(u128, i32), Error> {
            self.ensure_owner()?;

            let new_value = self.update_index_value()?;
            let performance = self.get_index_performance()?;

            Ok((new_value, performance))
        }

        /// Get index performance over time periods (if we had historical data)
        #[ink(message)]
        pub fn get_index_summary(&self) -> Result<(u128, u128, i32, u64), Error> {
            // Returns: (current_value, base_value, performance_bp, last_update)
            Ok((
                self.current_index_value,
                self.index_base_value,
                self.get_index_performance()?,
                self.last_index_update,
            ))
        }

        /// Enable/disable index tracking (owner only)
        #[ink(message)]
        pub fn set_index_tracking(&mut self, enabled: bool) -> Result<(), Error> {
            self.ensure_owner()?;

            self.index_tracking_enabled = enabled;

            if enabled && self.base_portfolio_value == 0 {
                // Auto-initialize if we have holdings
                if self.total_tokens_held > 0 {
                    self.initialize_base_portfolio_value()?;
                }
            }

            Ok(())
        }

        /// Check if index tracking is enabled
        #[ink(message)]
        pub fn is_index_tracking_enabled(&self) -> bool {
            self.index_tracking_enabled
        }

        /// Emergency reset index base value (owner only - use with extreme caution)
        #[ink(message)]
        pub fn emergency_reset_base_value(&mut self, reason: String) -> Result<(), Error> {
            self.ensure_owner()?;

            // Reset to current portfolio value as new baseline
            let current_value = self.calculate_total_portfolio_value()?;

            self.base_portfolio_value = current_value;
            self.current_index_value = self.index_base_value; // Reset to $100
            self.last_index_update = self.env().block_timestamp();

            // Emit reset event
            self.env().emit_event(BasePortfolioValueSet {
                base_value: self.index_base_value,
                base_portfolio_value: current_value,
                deployment_timestamp: self.env().block_timestamp(), // New timestamp
                set_by: self.env().caller(),
            });

            // Log the emergency reset
            self.env().emit_event(OperationFailed {
                operation: String::from("emergency_reset_base_value"),
                error: reason,
                caller: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Calculate performance in basis points vs base index value
        fn calculate_performance_bp(&self, current_value: u128) -> Result<i32, Error> {
            if self.index_base_value == 0 {
                return Ok(0);
            }

            // Calculate percentage change in basis points
            if current_value >= self.index_base_value {
                // Positive performance
                let gain = current_value.saturating_sub(self.index_base_value);
                let performance_bp = gain
                    .checked_mul(10000) // Convert to basis points
                    .ok_or(Error::InvalidParameter)?
                    .checked_div(self.index_base_value)
                    .ok_or(Error::InvalidParameter)?;

                // Convert to i32, capping at max value to prevent overflow
                Ok(performance_bp.min(i32::MAX as u128) as i32)
            } else {
                // Negative performance
                let loss = self.index_base_value.saturating_sub(current_value);
                let performance_bp = loss
                    .checked_mul(10000) // Convert to basis points
                    .ok_or(Error::InvalidParameter)?
                    .checked_div(self.index_base_value)
                    .ok_or(Error::InvalidParameter)?;

                // Return as negative, capping at min value and using safe conversion
                let capped_performance = performance_bp.min(i32::MAX as u128) as i32;
                let negative_performance = capped_performance.saturating_neg();
                Ok(negative_performance)
            }
        }

        /// Convert plancks to USD using DOT/USD oracle rate
        /// This will be fully implemented in Phase 4 with Oracle integration
        fn convert_plancks_to_usd(&self, plancks: u128) -> Result<u128, Error> {
            // Placeholder implementation - will integrate with Oracle in Phase 4
            // For now, assume 1 DOT = $6 USD (1 DOT = 10^10 plancks)
            // So $1 USD = 10^10 / 6 = ~1.67 × 10^9 plancks

            let placeholder_usd_rate = 1_666_666_667u128; // Plancks per USD (conservative estimate)

            if placeholder_usd_rate == 0 {
                return Err(Error::OracleCallFailed);
            }

            let usd_value = plancks.checked_div(placeholder_usd_rate).unwrap_or(0);
            Ok(usd_value)
        }

        // ===== INTEGRATION HOOKS FOR AUTOMATIC INDEX UPDATES =====

        /// Internal method to trigger index update after holdings change
        fn trigger_index_update(&mut self) {
            if self.index_tracking_enabled {
                // Update index value after any portfolio change
                let _ = self.update_index_value();
            }
        }

        // ===== PHASE 4A: REGISTRY INTEGRATION =====

        /// Cross-contract call to get token data from Registry
        fn call_registry_get_token_data(&self, token_id: u32) -> Result<EnrichedTokenData, Error> {
            let registry = self.registry_contract.ok_or_else(|| {
                self.emit_operation_failed(
                    "call_registry_get_token_data",
                    "Registry contract not set",
                );
                Error::InvalidParameter
            })?;

            let result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call(registry)
                .call_v1()
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("get_token_data"),
                    ))
                    .push_arg(token_id),
                )
                .returns::<Result<shared::EnrichedTokenData, shared::Error>>()
                .try_invoke();

            match result {
                Ok(registry_result) => match registry_result {
                    Ok(data) => {
                        let data = data?; // Unwrap the inner Result
                        Ok(EnrichedTokenData {
                            token_contract: data.token_contract,
                            oracle_contract: data.oracle_contract,
                            balance: data.balance,
                            weight_investment: data.weight_investment,
                            tier: data.tier,
                            market_cap: data.market_cap,
                            market_volume: data.market_volume,
                            price: data.price,
                        })
                    }
                    Err(_) => {
                        self.emit_operation_failed(
                            "call_registry_get_token_data",
                            "Registry returned error",
                        );
                        Err(Error::OracleCallFailed)
                    }
                },
                Err(_) => {
                    self.emit_operation_failed(
                        "call_registry_get_token_data",
                        "Registry call failed",
                    );
                    Err(Error::OracleCallFailed)
                }
            }
        }

        /// Cross-contract call to get active tier from Registry
        fn call_registry_get_active_tier(&self) -> Result<u32, Error> {
            let registry = self.registry_contract.ok_or_else(|| {
                self.emit_operation_failed(
                    "call_registry_get_active_tier",
                    "Registry contract not set",
                );
                Error::InvalidParameter
            })?;

            let result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call(registry)
                .call_v1()
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(ink::env::call::ExecutionInput::new(
                    ink::env::call::Selector::new(ink::selector_bytes!("get_active_tier")),
                ))
                .returns::<u32>() // Assuming Tier enum converts to u32
                .try_invoke();

            match result {
                Ok(tier_value) => match tier_value {
                    Ok(value) => Ok(value),
                    Err(_) => {
                        self.emit_operation_failed(
                            "call_registry_get_active_tier",
                            "Registry call returned error",
                        );
                        Err(Error::OracleCallFailed)
                    }
                },
                Err(_) => {
                    self.emit_operation_failed(
                        "call_registry_get_active_tier",
                        "Registry call failed",
                    );
                    Err(Error::OracleCallFailed)
                }
            }
        }

        /// Cross-contract call to get tokens by tier from Registry
        fn call_registry_get_tokens_by_tier(&self, tier: u32) -> Result<Vec<u32>, Error> {
            let registry = self.registry_contract.ok_or_else(|| {
                self.emit_operation_failed(
                    "call_registry_get_tokens_by_tier",
                    "Registry contract not set",
                );
                Error::InvalidParameter
            })?;

            let result = ink::env::call::build_call::<ink::env::DefaultEnvironment>()
                .call(registry)
                .call_v1()
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("get_tokens_by_tier"),
                    ))
                    .push_arg(tier),
                )
                .returns::<Vec<u32>>()
                .try_invoke();

            match result {
                Ok(token_ids) => match token_ids {
                    Ok(ids) => Ok(ids),
                    Err(_) => {
                        self.emit_operation_failed(
                            "call_registry_get_tokens_by_tier",
                            "Registry call returned error",
                        );
                        Err(Error::OracleCallFailed)
                    }
                },
                Err(_) => {
                    self.emit_operation_failed(
                        "call_registry_get_tokens_by_tier",
                        "Registry call failed",
                    );
                    Err(Error::OracleCallFailed)
                }
            }
        }

        /// Get real-time token price from Registry (public method for external use)
        #[ink(message)]
        pub fn get_token_market_data(&self, token_id: u32) -> Result<(u128, u128, u128), Error> {
            let token_data = self.call_registry_get_token_data(token_id)?;
            Ok((
                token_data.price,
                token_data.market_cap,
                token_data.market_volume,
            ))
        }

        /// Get current market value of a specific token holding
        #[ink(message)]
        pub fn get_token_holding_value(&self, token_id: u32) -> Result<u128, Error> {
            let holding = self.holdings.get(token_id).ok_or(Error::TokenNotFound)?;
            let token_data = self.call_registry_get_token_data(token_id)?;

            // Calculate value: amount × current_price
            let value = holding
                .amount
                .checked_mul(token_data.price)
                .ok_or(Error::InvalidParameter)?;
            Ok(value)
        }

        /// Get all holdings with current market values
        #[ink(message)]
        pub fn get_holdings_with_values(&self) -> Result<Vec<(u32, u128, u128)>, Error> {
            let mut holdings_with_values = Vec::new();

            for token_id in &self.held_token_ids {
                if let Some(holding) = self.holdings.get(*token_id) {
                    match self.call_registry_get_token_data(*token_id) {
                        Ok(token_data) => {
                            let value = holding.amount.checked_mul(token_data.price).unwrap_or(0);
                            holdings_with_values.push((*token_id, holding.amount, value));
                        }
                        Err(_) => {
                            // If we can't get price, include with 0 value
                            holdings_with_values.push((*token_id, holding.amount, 0));
                        }
                    }
                }
            }

            Ok(holdings_with_values)
        }

        /// Get active tier tokens for rebalancing decisions
        #[ink(message)]
        pub fn get_rebalancing_targets(&self) -> Result<Vec<u32>, Error> {
            let active_tier = self.call_registry_get_active_tier()?;
            self.call_registry_get_tokens_by_tier(active_tier)
        }

        /// Check if a token is in the active tier
        #[ink(message)]
        pub fn is_token_in_active_tier(&self, token_id: u32) -> Result<bool, Error> {
            let active_tier_tokens = self.get_rebalancing_targets()?;
            Ok(active_tier_tokens.contains(&token_id))
        }

        /// Validate portfolio holdings against Registry data
        #[ink(message)]
        pub fn validate_holdings_against_registry(&self) -> Result<Vec<u32>, Error> {
            let mut invalid_tokens = Vec::new();

            for token_id in &self.held_token_ids {
                // Try to get token data from Registry
                if self.call_registry_get_token_data(*token_id).is_err() {
                    invalid_tokens.push(*token_id);
                }
            }

            Ok(invalid_tokens)
        }

        /// Get portfolio composition with Registry market data
        #[ink(message)]
        pub fn get_portfolio_composition_with_market_data(
            &self,
        ) -> Result<PortfolioComposition, Error> {
            let mut holdings_vec = Vec::new();
            let mut total_value = 0u128;

            // Collect all holdings with market data
            for token_id in &self.held_token_ids {
                if let Some(holding) = self.holdings.get(*token_id) {
                    match self.call_registry_get_token_data(*token_id) {
                        Ok(token_data) => {
                            let token_value =
                                holding.amount.checked_mul(token_data.price).unwrap_or(0);
                            total_value = total_value.saturating_add(token_value);
                            holdings_vec.push((*token_id, holding));
                        }
                        Err(_) => {
                            // Include token even if we can't get market data
                            holdings_vec.push((*token_id, holding));
                        }
                    }
                }
            }

            // Add USDC balance to total value
            total_value = total_value.saturating_add(self.usdc_balance);

            Ok(PortfolioComposition {
                total_tokens: self.total_tokens_held,
                total_value,
                holdings: holdings_vec,
            })
        }

        // ===== UPDATED PORTFOLIO VALUE CALCULATIONS WITH REGISTRY DATA =====

        /// Calculate total portfolio value using real market data from Registry
        fn calculate_total_portfolio_value(&self) -> Result<u128, Error> {
            if self.total_tokens_held == 0 {
                return Ok(self.usdc_balance);
            }

            let mut total_value = 0u128;
            let mut successful_valuations = 0u32;

            // Calculate value of each token holding using Registry data
            for token_id in &self.held_token_ids {
                if let Some(holding) = self.holdings.get(*token_id) {
                    match self.call_registry_get_token_data(*token_id) {
                        Ok(token_data) => {
                            // Calculate: amount × current_price
                            let token_value =
                                holding.amount.checked_mul(token_data.price).unwrap_or(0);
                            total_value = total_value.saturating_add(token_value);
                            successful_valuations = successful_valuations.saturating_add(1);
                        }
                        Err(_) => {
                            // If Registry call fails, use fallback valuation
                            self.emit_operation_failed(
                                "calculate_total_portfolio_value",
                                &format!("Failed to get market data for token {}", token_id),
                            );

                            // Fallback: use amount as value (placeholder)
                            total_value = total_value.saturating_add(holding.amount);
                        }
                    }
                }
            }

            // Add USDC balance to total value
            total_value = total_value.saturating_add(self.usdc_balance);

            // Check if we got market data for most tokens
            if self.total_tokens_held > 0 && successful_valuations == 0 {
                // No successful Registry calls - this might indicate a problem
                self.emit_operation_failed(
                    "calculate_total_portfolio_value",
                    "No market data available from Registry",
                );
                return Err(Error::OracleCallFailed);
            }

            Ok(total_value)
        }

        /// Calculate portfolio value with fallback mechanisms
        fn calculate_portfolio_value_with_fallback(&self) -> u128 {
            // Try to get real market value first
            match self.calculate_total_portfolio_value() {
                Ok(value) => value,
                Err(_) => {
                    // Fallback: use token amounts as placeholder values
                    let mut fallback_value = 0u128;
                    for token_id in &self.held_token_ids {
                        if let Some(holding) = self.holdings.get(*token_id) {
                            fallback_value = fallback_value.saturating_add(holding.amount);
                        }
                    }
                    fallback_value.saturating_add(self.usdc_balance)
                }
            }
        }

        /// Get detailed portfolio valuation breakdown
        #[ink(message)]
        pub fn get_portfolio_valuation_breakdown(
            &self,
        ) -> Result<Vec<(u32, u128, u128, u128)>, Error> {
            // Returns: Vec<(token_id, amount_held, current_price, total_value)>
            let mut breakdown = Vec::new();

            for token_id in &self.held_token_ids {
                if let Some(holding) = self.holdings.get(*token_id) {
                    match self.call_registry_get_token_data(*token_id) {
                        Ok(token_data) => {
                            let total_value =
                                holding.amount.checked_mul(token_data.price).unwrap_or(0);
                            breakdown.push((
                                *token_id,
                                holding.amount,
                                token_data.price,
                                total_value,
                            ));
                        }
                        Err(_) => {
                            // Include with zero price if Registry call fails
                            breakdown.push((*token_id, holding.amount, 0, 0));
                        }
                    }
                }
            }

            Ok(breakdown)
        }

        /// Test Registry connection and data availability
        #[ink(message)]
        pub fn test_registry_connection(&self) -> Result<(bool, u32), Error> {
            if self.registry_contract.is_none() {
                return Ok((false, 0));
            }

            let mut successful_calls = 0u32;
            let _test_count = self.held_token_ids.len().min(5) as u32; // Test up to 5 tokens

            for token_id in self.held_token_ids.iter().take(5) {
                if self.call_registry_get_token_data(*token_id).is_ok() {
                    successful_calls = successful_calls.saturating_add(1);
                }
            }

            Ok((successful_calls > 0, successful_calls))
        }

        // ===== ERROR HANDLING HELPER =====

        /// Emit operation failed event for monitoring
        fn emit_operation_failed(&self, operation: &str, error: &str) {
            self.env().emit_event(OperationFailed {
                operation: String::from(operation),
                error: String::from(error),
                caller: self.env().caller(),
                timestamp: self.env().block_timestamp(),
            });
        }
    }
}
