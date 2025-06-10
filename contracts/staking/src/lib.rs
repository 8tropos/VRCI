// staking/src/lib.rs

#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod tests;
pub mod unstaking_request;

#[ink::contract]
mod w3pi_staking {
    use crate::unstaking_request::UnstakingRequest;
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use shared::errors::Error;
    use shared::non_reentrant;
    use shared::tier::Tier;
    use shared::utils::reentrancy_guard::ReentrancyGuard;
    use core::convert::TryFrom;

    // Constants
    pub const MAX_UNSTAKING_REQUESTS: u32 = 10;
    pub const REWARDS_RATE_ANNUAL: u128 = 5_000_000_000; // 5% APR (5% * 10^8)
    pub const SECONDS_PER_YEAR: u64 = 31_536_000; // 365 days in seconds
    pub const PERFORMANCE_FEE_PERCENT: u128 = 10; // Staking fee: 10% of rewards

    // Tier-based unstaking periods (in seconds)
    pub const TIER1_UNSTAKING_PERIOD: u64 = 14 * 24 * 60 * 60; // 14 days
    pub const TIER2_UNSTAKING_PERIOD: u64 = 10 * 24 * 60 * 60; // 10 days
    pub const TIER3_UNSTAKING_PERIOD: u64 = 7 * 24 * 60 * 60; // 7 days
    pub const TIER4_UNSTAKING_PERIOD: u64 = 3 * 24 * 60 * 60; // 3 days

    // Events

    /// Event emitted when tokens are staked
    #[ink(event)]
    pub struct Staked {
        #[ink(topic)]
        pub account: AccountId,
        pub amount: u128,
        pub unstaking_period: u64,
    }

    /// Event emitted when an unstaking request is created
    #[ink(event)]
    pub struct UnstakeRequested {
        #[ink(topic)]
        pub account: AccountId,
        pub amount: u128,
        pub available_at: u64,
    }

    /// Event emitted when unstaked tokens are claimed
    #[ink(event)]
    pub struct UnstakedClaimed {
        #[ink(topic)]
        pub account: AccountId,
        pub amount: u128,
    }

    /// Event emitted when rewards are claimed
    #[ink(event)]
    pub struct RewardsClaimed {
        #[ink(topic)]
        pub account: AccountId,
        pub amount: u128,
    }

    /// Event emitted when the contract is paused
    #[ink(event)]
    pub struct ContractPaused {
        #[ink(topic)]
        pub by: AccountId,
    }

    /// Event emitted when the contract is unpaused
    #[ink(event)]
    pub struct ContractUnpaused {
        #[ink(topic)]
        pub by: AccountId,
    }

    /// Event emitted when performance fees are claimed
    #[ink(event)]
    pub struct PerformanceFeeClaimed {
        #[ink(topic)]
        pub account: AccountId,
        pub fee_amount: u128,
    }

    /// Main stake information structure
    #[derive(Debug, scale::Encode, scale::Decode, Clone)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct StakeInfo {
        /// Amount staked
        pub amount: u128,
        /// When the stake was created
        pub staked_at: u64,
        /// Last reward claim time
        pub last_claim: u64,
        /// Unstaking period for this stake (in seconds)
        pub unstaking_period: u64,
        /// Current tier when staked (for reference)
        pub tier_at_stake: Tier,
    }

    /// Staking contract storage
    #[ink(storage)]
    pub struct W3piStaking {
        /// The W3PI token contract address
        w3pi_token: AccountId,
        /// The registry contract address (for tier info)
        registry: AccountId,
        /// Contract owner
        owner: AccountId,
        /// Contract pause state
        paused: bool,
        /// Stakes per account
        stakes: Mapping<AccountId, StakeInfo>,
        /// Total staked amount
        total_staked: u128,
        /// Unstaking requests per account
        unstaking_requests: Mapping<AccountId, Vec<UnstakingRequest>>,
        /// Reentrancy guard
        reentrancy_guard: ReentrancyGuard,
        /// Fee wallet for collected performance fees
        fee_wallet: AccountId,
        /// Total collected fees
        total_collected_fees: u128,
    }

    impl W3piStaking {
        /// Constructor that initializes the staking contract
        #[ink(constructor)]
        pub fn new(w3pi_token: AccountId, registry: AccountId, fee_wallet: AccountId) -> Self {
            Self {
                w3pi_token,
                registry,
                owner: Self::env().caller(),
                paused: false,
                stakes: Mapping::default(),
                total_staked: 0,
                unstaking_requests: Mapping::default(),
                reentrancy_guard: ReentrancyGuard::new(),
                fee_wallet,
                total_collected_fees: 0,
            }
        }

        /// Ensure the contract is not paused
        fn ensure_not_paused(&self) -> Result<(), Error> {
            if self.paused {
                return Err(Error::ContractPaused);
            }
            Ok(())
        }

        /// Ensure the caller is the owner
        fn ensure_owner(&self) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        /// Get unstaking period based on the current active tier
        fn get_unstaking_period(&self) -> Result<u64, Error> {
            // Call registry to get current tier
            let current_tier = self.get_current_tier()?;

            // Return unstaking period based on tier
            let unstaking_period = match current_tier {
                Tier::Tier1 => TIER1_UNSTAKING_PERIOD,
                Tier::Tier2 => TIER2_UNSTAKING_PERIOD,
                Tier::Tier3 => TIER3_UNSTAKING_PERIOD,
                Tier::Tier4 => TIER4_UNSTAKING_PERIOD,
                Tier::None => TIER1_UNSTAKING_PERIOD, // Default to longest period
            };

            Ok(unstaking_period)
        }

        /// Get current tier from registry
        fn get_current_tier(&self) -> Result<Tier, Error> {
            use ink::env::call::{build_call, ExecutionInput, Selector};
            use ink::env::DefaultEnvironment;

            // Call the registry contract to get current tier
            match build_call::<DefaultEnvironment>()
                .call(self.registry)
                .exec_input(ExecutionInput::new(Selector::new([0x9B, 0x4F, 0x62, 0x31]))) // get_current_tier selector
                .returns::<Tier>()
                .try_invoke()
            {
                Ok(result) => match result {
                    Ok(tier) => Ok(tier),
                    Err(_) => Err(Error::CrossContractCallFailed),
                },
                Err(_) => Err(Error::CrossContractCallFailed),
            }
        }

        /// Calculate rewards for a stake
        fn calculate_rewards(&self, stake: &StakeInfo) -> u128 {
            let current_time = self.env().block_timestamp();

            // Time since last claim
            let time_elapsed = current_time.saturating_sub(stake.last_claim);

            // Handle zero time elapsed case
            if time_elapsed == 0 {
                return 0;
            }

            // Convert to u128 for calculation
            let time_elapsed_u128 = time_elapsed as u128;
            let seconds_per_year_u128 = SECONDS_PER_YEAR as u128;

            // Calculate reward: amount * rate * time_elapsed / seconds_per_year / 10^8
            stake
                .amount
                .saturating_mul(REWARDS_RATE_ANNUAL)
                .saturating_mul(time_elapsed_u128)
                .checked_div(seconds_per_year_u128)
                .unwrap_or(0)
                .checked_div(100_000_000)
                .unwrap_or(0)
        }

        /// Transfer tokens from caller to contract
        fn transfer_tokens_to_contract(&self, from: AccountId, amount: u128) -> Result<(), Error> {
            use ink::env::call::{build_call, ExecutionInput, Selector};
            use ink::env::DefaultEnvironment;

            // Call the token contract to transfer tokens
            // Using correct selector 0x0B396F18 for transfer_from
            build_call::<DefaultEnvironment>()
                .call(self.w3pi_token)
                .exec_input(
                    ExecutionInput::new(Selector::new([0x0B, 0x39, 0x6F, 0x18])) // transfer_from selector
                        .push_arg(from)
                        .push_arg(self.env().account_id())
                        .push_arg(amount),
                )
                .returns::<Result<(), Error>>()
                .try_invoke()
                .map_err(|_| Error::TransferFailed)? // Handle LangError
                .map_err(|_| Error::TransferFailed)? // Handle contract error
        }

        /// Calculate rewards with performance fee
        /// Returns (net_reward, fee_amount)
        fn calculate_rewards_with_fee(&self, stake: &StakeInfo) -> (u128, u128) {
            let current_time = self.env().block_timestamp();

            // Time since last claim
            let time_elapsed = current_time.saturating_sub(stake.last_claim);

            // Handle zero time elapsed case
            if time_elapsed == 0 {
                return (0, 0);
            }

            // Convert to u128 for calculation
            let time_elapsed_u128 = time_elapsed as u128;
            let seconds_per_year_u128 = SECONDS_PER_YEAR as u128;

            // Calculate total reward: amount * rate * time_elapsed / seconds_per_year / 10^8
            let total_reward = stake
                .amount
                .saturating_mul(REWARDS_RATE_ANNUAL)
                .saturating_mul(time_elapsed_u128)
                .checked_div(seconds_per_year_u128)
                .unwrap_or(0)
                .checked_div(100_000_000)
                .unwrap_or(0);

            // Calculate performance fee (10% of rewards)
            let fee_amount = total_reward
                .saturating_mul(PERFORMANCE_FEE_PERCENT)
                .checked_div(100)
                .unwrap_or(0);

            // Net reward is total minus fee
            let net_reward = total_reward.saturating_sub(fee_amount);

            (net_reward, fee_amount)
        }

        /// Transfer tokens from contract to recipient
        fn transfer_tokens_from_contract(&self, to: AccountId, amount: u128) -> Result<(), Error> {
            use ink::env::call::{build_call, ExecutionInput, Selector};
            use ink::env::DefaultEnvironment;

            // Call the token contract to transfer tokens
            // Using correct selector 0x84A15DA1 for transfer
            build_call::<DefaultEnvironment>()
                .call(self.w3pi_token)
                .exec_input(
                    ExecutionInput::new(Selector::new([0x84, 0xA1, 0x5D, 0xA1])) // transfer selector
                        .push_arg(to)
                        .push_arg(amount),
                )
                .returns::<Result<(), Error>>()
                .try_invoke()
                .map_err(|_| Error::TransferFailed)? // Handle LangError
                .map_err(|_| Error::TransferFailed)? // Handle contract error
        }

        /// Stake W3PI tokens
        #[ink(message)]
        pub fn stake(&mut self, amount: u128) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_not_paused()?;

                let caller = self.env().caller();
                let current_time = self.env().block_timestamp();

                if amount == 0 {
                    return Err(Error::InvalidParameters);
                }

                // Get unstaking period based on current tier
                let unstaking_period = self.get_unstaking_period()?;
                let current_tier = self.get_current_tier()?;

                // Check if user already has a stake
                let stake_info = if let Some(existing_stake) = self.stakes.get(caller) {
                    // Calculate pending rewards and fee
                    let (net_reward, fee_amount) = self.calculate_rewards_with_fee(&existing_stake);

                    // Update total fees collected
                    if fee_amount > 0 {
                        self.total_collected_fees =
                            self.total_collected_fees.saturating_add(fee_amount);

                        // Transfer fee to fee wallet
                        self.transfer_tokens_from_contract(self.fee_wallet, fee_amount)?;

                        // Emit fee event
                        self.env().emit_event(PerformanceFeeClaimed {
                            account: caller,
                            fee_amount,
                        });
                    }

                    // Update stake info
                    let new_amount = existing_stake.amount.saturating_add(amount);

                    // Add pending net rewards to stake amount (auto-compound)
                    let new_amount_with_rewards = new_amount.saturating_add(net_reward);

                    StakeInfo {
                        amount: new_amount_with_rewards,
                        staked_at: existing_stake.staked_at,
                        last_claim: current_time,
                        unstaking_period,
                        tier_at_stake: current_tier,
                    }
                } else {
                    // Create new stake info
                    StakeInfo {
                        amount,
                        staked_at: current_time,
                        last_claim: current_time,
                        unstaking_period,
                        tier_at_stake: current_tier,
                    }
                };

                // Update storage
                self.stakes.insert(caller, &stake_info);
                self.total_staked = self.total_staked.saturating_add(amount);

                // Transfer tokens from caller to contract
                self.transfer_tokens_to_contract(caller, amount)?;

                // Emit event
                self.env().emit_event(Staked {
                    account: caller,
                    amount,
                    unstaking_period,
                });

                Ok(())
            })
        }

        /// Request to unstake tokens
        #[ink(message)]
        pub fn request_unstake(&mut self, amount: u128) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_not_paused()?;

                let caller = self.env().caller();
                let current_time = self.env().block_timestamp();

                if amount == 0 {
                    return Err(Error::InvalidParameters);
                }

                // Get stake info
                let mut stake_info = self.stakes.get(caller).ok_or(Error::InvalidParameters)?;

                // Check if sufficient stake
                if stake_info.amount < amount {
                    return Err(Error::InsufficientBalance);
                }

                // Check if unstaking requests limit reached
                let mut requests = self.unstaking_requests.get(caller).unwrap_or_default();
                let requests_len = u32::try_from(requests.len()).map_err(|_| Error::InvalidParameters)?;
                if requests_len >= MAX_UNSTAKING_REQUESTS {
                    return Err(Error::InvalidParameters);
                }

                // Update stake amount
                stake_info.amount = stake_info.amount.saturating_sub(amount);

                // Create unstaking request
                let available_at = current_time.saturating_add(stake_info.unstaking_period);
                let request = UnstakingRequest {
                    amount,
                    requested_at: current_time,
                    available_at,
                    claimed: false,
                };

                // Update storage
                requests.push(request);
                self.unstaking_requests.insert(caller, &requests);

                if stake_info.amount == 0 {
                    // Remove stake if amount is 0
                    self.stakes.remove(caller);
                } else {
                    // Update stake info
                    self.stakes.insert(caller, &stake_info);
                }

                self.total_staked = self.total_staked.saturating_sub(amount);

                // Emit event
                self.env().emit_event(UnstakeRequested {
                    account: caller,
                    amount,
                    available_at,
                });

                Ok(())
            })
        }

        /// Claim unstaked tokens that have completed the unstaking period
        #[ink(message)]
        pub fn claim_unstaked(&mut self) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_not_paused()?;

                let caller = self.env().caller();
                let current_time = self.env().block_timestamp();

                // Get unstaking requests
                let mut requests = self.unstaking_requests.get(caller).unwrap_or_default();

                if requests.is_empty() {
                    return Err(Error::InvalidParameters);
                }

                let mut total_to_claim: u128 = 0; // Explicitly define type as u128
                let mut has_claimable = false;

                // Process each request
                for request in requests.iter_mut() {
                    if !request.claimed && current_time >= request.available_at {
                        total_to_claim = total_to_claim.saturating_add(request.amount);
                        request.claimed = true;
                        has_claimable = true;
                    }
                }

                if !has_claimable {
                    return Err(Error::InvalidParameters);
                }

                // Update storage
                self.unstaking_requests.insert(caller, &requests);

                // Transfer tokens
                self.transfer_tokens_from_contract(caller, total_to_claim)?;

                // Emit event
                self.env().emit_event(UnstakedClaimed {
                    account: caller,
                    amount: total_to_claim,
                });

                Ok(())
            })
        }

        /// Claim staking rewards without unstaking
        #[ink(message)]
        pub fn claim_rewards(&mut self) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_not_paused()?;

                let caller = self.env().caller();
                let current_time = self.env().block_timestamp();

                // Get stake info
                let mut stake_info = self.stakes.get(caller).ok_or(Error::InvalidParameters)?;

                // Calculate rewards and fee
                let (net_reward, fee_amount) = self.calculate_rewards_with_fee(&stake_info);

                if net_reward == 0 {
                    return Err(Error::InvalidParameters);
                }

                // Update last claim time
                stake_info.last_claim = current_time;
                self.stakes.insert(caller, &stake_info);

                // Update total fees collected
                self.total_collected_fees = self.total_collected_fees.saturating_add(fee_amount);

                // Transfer net rewards to user
                self.transfer_tokens_from_contract(caller, net_reward)?;

                // Transfer fee to fee wallet (if fee is non-zero)
                if fee_amount > 0 {
                    self.transfer_tokens_from_contract(self.fee_wallet, fee_amount)?;

                    // Emit fee event
                    self.env().emit_event(PerformanceFeeClaimed {
                        account: caller,
                        fee_amount,
                    });
                }

                // Emit reward event
                self.env().emit_event(RewardsClaimed {
                    account: caller,
                    amount: net_reward,
                });

                Ok(())
            })
        }

        /// View function to get claimable rewards
        #[ink(message)]
        pub fn get_claimable_rewards(&self, account: AccountId) -> u128 {
            if let Some(stake_info) = self.stakes.get(account) {
                let (net_reward, _) = self.calculate_rewards_with_fee(&stake_info);
                net_reward
            } else {
                0
            }
        }

        // Getter for total collected fees
        #[ink(message)]
        pub fn get_total_collected_fees(&self) -> u128 {
            self.total_collected_fees
        }

        // Function to update fee wallet
        #[ink(message)]
        pub fn set_fee_wallet(&mut self, new_fee_wallet: AccountId) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_owner()?;
                self.fee_wallet = new_fee_wallet;
                Ok(())
            })
        }

        /// View function to get account stake info
        #[ink(message)]
        pub fn get_stake_info(&self, account: AccountId) -> Option<StakeInfo> {
            self.stakes.get(account)
        }

        /// View function to get unstaking requests
        #[ink(message)]
        pub fn get_unstaking_requests(&self, account: AccountId) -> Vec<UnstakingRequest> {
            self.unstaking_requests.get(account).unwrap_or_default()
        }

        /// View function to get total staked amount
        #[ink(message)]
        pub fn get_total_staked(&self) -> u128 {
            self.total_staked
        }

        /// Pause the contract (owner only)
        #[ink(message)]
        pub fn pause(&mut self) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_owner()?;
                if self.paused {
                    return Ok(());
                }
                self.paused = true;
                self.env().emit_event(ContractPaused {
                    by: self.env().caller(),
                });
                Ok(())
            })
        }

        /// Unpause the contract (owner only)
        #[ink(message)]
        pub fn unpause(&mut self) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_owner()?;
                if !self.paused {
                    return Ok(());
                }
                self.paused = false;
                self.env().emit_event(ContractUnpaused {
                    by: self.env().caller(),
                });
                Ok(())
            })
        }

        /// Update the W3PI token address (owner only)
        #[ink(message)]
        pub fn set_w3pi_token(&mut self, new_token: AccountId) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_owner()?;
                self.w3pi_token = new_token;
                Ok(())
            })
        }

        /// Update the registry address (owner only)
        #[ink(message)]
        pub fn set_registry(&mut self, new_registry: AccountId) -> Result<(), Error> {
            non_reentrant!(self, {
                self.ensure_owner()?;
                self.registry = new_registry;
                Ok(())
            })
        }
    }
}
