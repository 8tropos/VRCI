#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::vec::Vec;
use ink::storage::traits::StorageLayout;
use ink::storage::Mapping;
use shared::errors::Error;
use shared::utils::reentrancy_guard::ReentrancyGuard;

#[ink::contract]
mod hydradx_dex {
    use super::*;
    use shared::non_reentrant;

    /// Simple pool structure for demonstration
    #[derive(scale::Encode, scale::Decode, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
    pub struct Pool {
        pub token_a: AccountId,
        pub token_b: AccountId,
        pub reserve_a: u128,
        pub reserve_b: u128,
    }

    #[ink(event)]
    pub struct SwapExecuted {
        #[ink(topic)]
        pub from: AccountId,
        #[ink(topic)]
        pub to: AccountId,
        pub amount_in: u128,
        pub amount_out: u128,
    }

    #[ink(storage)]
    pub struct HydraDxDex {
        /// Pools indexed by (token_a, token_b)
        pools: Mapping<(AccountId, AccountId), Pool>,
        /// Owner for admin functions
        owner: AccountId,
        reentrancy_guard: ReentrancyGuard,
        pool_keys: Vec<(AccountId, AccountId)>,
    }

    impl HydraDxDex {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                pools: Mapping::default(),
                owner: Self::env().caller(),
                reentrancy_guard: ReentrancyGuard::new(),
                pool_keys: Vec::new(),
            }
        }

        /// Admin: Add or update a pool (for demo/testing)
        #[ink(message)]
        pub fn set_pool(
            &mut self,
            token_a: AccountId,
            token_b: AccountId,
            reserve_a: u128,
            reserve_b: u128,
        ) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }
            let pool = Pool {
                token_a,
                token_b,
                reserve_a,
                reserve_b,
            };
            self.pools.insert((token_a, token_b), &pool);
            self.pool_keys.push((token_a, token_b));
            Ok(())
        }

        /// Swap tokens from one to another
        #[ink(message, selector = 0x0D0E0F10)]
        pub fn swap(
            &mut self,
            from: AccountId,
            to: AccountId,
            amount: u128,
            path: Vec<AccountId>,
        ) -> Result<u128, Error> {
            non_reentrant!(self, {
                if path.len() != 2 || path[0] != from || path[1] != to {
                    return Err(Error::InvalidParameters);
                }
                let mut pool = self
                    .pools
                    .get((from, to))
                    .or_else(|| self.pools.get((to, from)))
                    .ok_or(Error::TokenNotFound)?;
                let (reserve_in, reserve_out) = if pool.token_a == from {
                    (&mut pool.reserve_a, &mut pool.reserve_b)
                } else {
                    (&mut pool.reserve_b, &mut pool.reserve_a)
                };
                if *reserve_in < amount || *reserve_in == 0 || *reserve_out == 0 {
                    return Err(Error::InsufficientBalance);
                }
                // x * y = k, dy = (y * dx) / (x + dx)
                let amount_out = (*reserve_out as u128).saturating_mul(amount)
                    / ((*reserve_in as u128).saturating_add(amount));
                *reserve_in = reserve_in.saturating_add(amount);
                *reserve_out = reserve_out.saturating_sub(amount_out);
                self.pools.insert((pool.token_a, pool.token_b), &pool);
                self.env().emit_event(SwapExecuted {
                    from,
                    to,
                    amount_in: amount,
                    amount_out,
                });
                Ok(amount_out)
            })
        }

        /// Get token price
        #[ink(message, selector = 0x11121314)]
        pub fn get_token_price(&self, token: AccountId) -> Result<u128, Error> {
            for key in &self.pool_keys {
                if let Some(pool) = self.pools.get(*key) {
                    if pool.token_a == token && pool.reserve_b > 0 {
                        return Ok(pool.reserve_a / pool.reserve_b);
                    } else if pool.token_b == token && pool.reserve_a > 0 {
                        return Ok(pool.reserve_b / pool.reserve_a);
                    }
                }
            }
            Err(Error::TokenNotFound)
        }
    }
}
