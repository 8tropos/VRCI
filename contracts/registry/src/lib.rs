// w3pi/contracts/registry/src/lib.rs

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod registry {
    use ink::storage::Mapping;
    use shared::{EnrichedTokenData, Error, Oracle, TokenData};

    #[ink(storage)]
    pub struct Registry {
        /// Mapping from token ID to token data
        tokens: Mapping<u32, TokenData>,
        /// Next available token ID
        next_token_id: u32,
        /// Registry owner
        owner: AccountId,
    }

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

    impl Default for Registry {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Registry {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                tokens: Mapping::default(),
                next_token_id: 1,
                owner: Self::env().caller(),
            }
        }

        /// Add a new token to the registry
        #[ink(message)]
        pub fn add_token(
            &mut self,
            token_contract: AccountId,
            oracle_contract: AccountId,
        ) -> Result<u32, Error> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }

            let token_id = self.next_token_id;
            let token_data = TokenData {
                token_contract,
                oracle_contract,
                balance: 0,
                weight_investment: 0,
                tier: 0,
            };

            self.tokens.insert(token_id, &token_data);
            self.next_token_id = self.next_token_id.saturating_add(1);

            self.env().emit_event(TokenAdded {
                token_id,
                token_contract,
                oracle_contract,
            });

            Ok(token_id)
        }

        /// Get token data with live oracle prices
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
                tier: token_data.tier,
                market_cap,
                market_volume,
                price,
            };

            Ok(enriched_data)
        }

        /// Update token balance and investment data
        #[ink(message)]
        pub fn update_token(
            &mut self,
            token_id: u32,
            balance: u128,
            weight_investment: u32,
            tier: u32,
        ) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }

            let mut token_data = self.tokens.get(token_id).ok_or(Error::TokenNotFound)?;

            token_data.balance = balance;
            token_data.weight_investment = weight_investment;
            token_data.tier = tier;

            self.tokens.insert(token_id, &token_data);

            self.env().emit_event(TokenUpdated {
                token_id,
                balance,
                weight_investment,
                tier,
            });

            Ok(())
        }

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

        /// Get basic token data without oracle calls
        #[ink(message)]
        pub fn get_basic_token_data(&self, token_id: u32) -> Result<TokenData, Error> {
            self.tokens.get(token_id).ok_or(Error::TokenNotFound)
        }
    }
}
