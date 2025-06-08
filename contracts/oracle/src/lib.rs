#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod oracle {
    use ink::storage::Mapping;
    use shared::Error;

    #[ink(storage)]
    pub struct Oracle {
        /// Price data for tokens in plancks
        prices: Mapping<AccountId, u128>,
        /// Market cap data in plancks
        market_caps: Mapping<AccountId, u128>,
        /// Market volume data in plancks  
        market_volumes: Mapping<AccountId, u128>,
        /// Contract owner
        owner: AccountId,
    }

    #[ink(event)]
    pub struct PriceUpdated {
        #[ink(topic)]
        token: AccountId,
        price: u128,
    }

    #[ink(event)]
    pub struct MarketDataUpdated {
        #[ink(topic)]
        token: AccountId,
        market_cap: u128,
        volume: u128,
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
                prices: Mapping::default(),
                market_caps: Mapping::default(),
                market_volumes: Mapping::default(),
                owner: caller,
            }
        }

        /// Constructor with sample data
        #[ink(constructor)]
        pub fn new_with_data() -> Self {
            let mut oracle = Self::new();

            // Set default prices in plancks (1 DOT = 10^10 plancks)
            let dummy_token = AccountId::from([0x01; 32]);
            oracle.prices.insert(dummy_token, &10_000_000_000); // 1 DOT
            oracle
                .market_caps
                .insert(dummy_token, &1_000_000_000_000_000); // 100,000 DOT
            oracle
                .market_volumes
                .insert(dummy_token, &100_000_000_000_000); // 10,000 DOT

            oracle
        }

        /// Get token price
        #[ink(message)]
        pub fn get_price(&self, token: AccountId) -> Option<u128> {
            self.prices.get(token)
        }

        /// Get market cap
        #[ink(message)]
        pub fn get_market_cap(&self, token: AccountId) -> Option<u128> {
            self.market_caps.get(token)
        }

        /// Get market volume
        #[ink(message)]
        pub fn get_market_volume(&self, token: AccountId) -> Option<u128> {
            self.market_volumes.get(token)
        }

        /// Update price data (owner only)
        #[ink(message)]
        pub fn update_price(&mut self, token: AccountId, price: u128) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }

            self.prices.insert(token, &price);
            self.env().emit_event(PriceUpdated { token, price });

            Ok(())
        }

        /// Update market data (owner only)
        #[ink(message)]
        pub fn update_market_data(
            &mut self,
            token: AccountId,
            market_cap: u128,
            volume: u128,
        ) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }

            self.market_caps.insert(token, &market_cap);
            self.market_volumes.insert(token, &volume);

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
    }
}
