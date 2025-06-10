// staking/src/tests.rs

#[cfg(test)]
mod tests {
    use crate::w3pi_staking::W3piStaking;
    use ink::env::{DefaultEnvironment, Environment};
    use shared::errors::Error;

    // Helper function to set up a test contract
    fn create_contract() -> W3piStaking {
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        // Using bob as the w3pi token, charlie as the registry, and django as the fee wallet
        W3piStaking::new(accounts.bob, accounts.charlie, accounts.django)
    }

    #[ink::test]
    fn test_constructor() {
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        let contract = W3piStaking::new(accounts.bob, accounts.charlie, accounts.django);

        // Check initial state
        assert_eq!(contract.get_total_staked(), 0);
        assert_eq!(contract.get_total_collected_fees(), 0);
    }

    // Test basic admin functions - not including pause/unpause
    #[ink::test]
    fn test_basic_admin() {
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        let mut contract = create_contract();

        // Set caller as Alice (owner)
        ink::env::test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Test fee wallet update
        let result = contract.set_fee_wallet(accounts.eve);
        assert!(result.is_ok(), "Owner should be able to set fee wallet");

        // Test W3PI token update
        let result = contract.set_w3pi_token(accounts.frank);
        assert!(
            result.is_ok(),
            "Owner should be able to set W3PI token address"
        );

        // Test registry update
        let result = contract.set_registry(accounts.django);
        assert!(
            result.is_ok(),
            "Owner should be able to set registry address"
        );
    }

    // Test only pause, not unpause
    #[ink::test]
    fn test_pause() {
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        let mut contract = create_contract();

        // Set caller as Alice (owner)
        ink::env::test::set_caller::<DefaultEnvironment>(accounts.alice);

        // Test pause function only
        let result = contract.pause();
        assert!(result.is_ok(), "Owner should be able to pause");
    }

    // Try to test unpause separately
    // Note: This test may fail
    #[ink::test]
    fn test_unpause() {
        let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
        let mut contract = create_contract();

        // Set caller as Alice (owner)
        ink::env::test::set_caller::<DefaultEnvironment>(accounts.alice);

        // First pause
        let pause_result = contract.pause();
        assert!(pause_result.is_ok(), "Should be able to pause first");

        // This is where the error is occurring
        let unpause_result = contract.unpause();

        // For debugging, let's also check the error if it fails
        if unpause_result.is_err() {
            match unpause_result {
                Err(Error::Unauthorized) => panic!("Failed with Unauthorized error"),
                Err(Error::ContractPaused) => panic!("Failed with ContractPaused error"),
                Err(Error::ReentrantCall) => panic!("Failed with ReentrantCall error"),
                _ => panic!("Failed with some other error"),
            }
        }

        assert!(unpause_result.is_ok(), "Owner should be able to unpause");
    }
}
