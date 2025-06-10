# W3PI Registry Contract Testing Guide
## Complete 22-Phase Testing Plan for Tier System

This comprehensive testing guide validates all new tier system functionality while ensuring backward compatibility and proper error handling.

---

## **Phase 1: Initial Setup & Configuration**

### 1. **Set Up DOT/USD Oracle** (Critical First Step)
```bash
# Call on Registry Contract
registry.set_dot_usd_oracle(oracle_contract_address)

# Verify setup
registry.get_dot_usd_oracle() 
# Expected: Returns your oracle contract address
```

**Purpose**: Enable USD-to-plancks conversion for tier calculations

### 2. **Grant Roles for Testing**
```bash
# Call on Registry Contract
registry.grant_role(Role::TokenManager, your_test_account)
registry.grant_role(Role::TokenUpdater, your_test_account)

# Verify roles
registry.has_role(Role::TokenManager, your_account)
# Expected: Returns true
```

**Purpose**: Allow your account to manage tokens during testing

### 3. **Verify Initial Configuration**
```bash
# Check default state
registry.get_active_tier()
# Expected: Tier1 (default)

registry.get_tier_thresholds()
# Expected: Returns default USD thresholds (Tier1: $50M/$5M, etc.)

registry.get_tier_distribution()
# Expected: All tiers show count of 0 initially
```

---

## **Phase 2: Token Registration & Tier Classification**

### 4. **Add First Token**
```bash
# Add token with oracle
registry.add_token(token_contract_1, oracle_contract_1)
# Expected: Returns token_id = 1

# Verify token added
registry.get_enhanced_token_data(1)
# Expected: Shows token data with assigned tier

registry.get_tier_distribution()
# Expected: Count increased for the assigned tier

# Check events
# Expected: TokenAdded event emitted with initial_tier
```

**Purpose**: Test basic token registration and automatic tier assignment

### 5. **Test Tier Calculation**
```bash
# Calculate tier for token
registry.calculate_token_tier(1)
# Expected: Returns tier based on oracle market cap/volume

# Compare with stored data
registry.get_enhanced_token_data(1).tier
# Expected: Should match calculate_token_tier result
```

**Purpose**: Verify tier calculation logic works correctly

### 6. **Add Multiple Tokens** (3-5 tokens minimum)
```bash
# Repeat for different tokens
registry.add_token(token_contract_2, oracle_contract_2)
registry.add_token(token_contract_3, oracle_contract_3)
registry.add_token(token_contract_4, oracle_contract_4)
registry.add_token(token_contract_5, oracle_contract_5)

# Check distribution after each
registry.get_tier_distribution()
# Expected: Distribution changes with each addition
```

**Purpose**: Create diverse tier distribution for 80% rule testing

---

## **Phase 3: Tier Management & Grace Periods**

### 7. **Test Manual Tier Updates (Enhanced with New Features)**
```bash
# Force tier recalculation
registry.update_token_tier(token_id)
# Expected: Returns updated tier

# Check for events
# Expected: TokenTierChanged event if tier changed OR GracePeriodStarted if grace period initiated

# Verify updated data
registry.get_enhanced_token_data(token_id)
# Expected: Shows new tier information or pending tier change

# NEW: Check grace period status
registry.get_grace_period_remaining(token_id)
# Expected: Returns remaining time in milliseconds (e.g., 7776000000 for 90 days)

registry.get_grace_period_end_time(token_id)
# Expected: Returns timestamp when grace period ends

registry.is_grace_period_expired(token_id)
# Expected: false (if within grace period)
```

**Purpose**: Test manual tier recalculation and new grace period monitoring

### 8. **Test Grace Period System (Enhanced)**
```bash
# Prerequisites: Ensure oracle data changes for a token
# Modify oracle market cap to trigger tier change

# Update token to trigger tier change
registry.update_token(token_id, new_balance, weight)
# Expected: Should trigger pending tier change

# Check pending changes
registry.get_tokens_with_pending_changes()
# Expected: Shows token with pending tier change

# NEW: Monitor grace period progress
registry.get_grace_period_remaining(token_id)
# Expected: Shows time remaining (should be close to full grace period)

registry.get_grace_period_days()
# Expected: Shows current grace period in days (default: 90)

# Verify grace period started
# Expected: GracePeriodStarted event emitted with grace_end_time
```

**Purpose**: Test grace period mechanism with enhanced monitoring

### 9. **Process Grace Periods (Enhanced)**
```bash
# Note: In production, wait for grace period to expire
# For testing, you can adjust grace period first

# NEW: Adjust grace period for testing
registry.set_grace_period(300000)  # 5 minutes for testing
# Expected: Success (owner only), GracePeriodUpdated event

# Verify grace period change
registry.get_grace_period_hours()
# Expected: 0 (since 5 minutes < 1 hour)

registry.get_grace_period() / 60000  # Convert to minutes
# Expected: 5

# Wait for grace period to expire, then process
registry.process_grace_periods()
# Expected: Returns count of processed tokens

# Verify changes applied
registry.get_tokens_with_pending_changes()
# Expected: Should be empty after processing

# Check events
# Expected: TokenTierChanged events with reason "grace_period_ended"
```

**Purpose**: Test grace period expiration with adjustable timing

### **NEW 9b. Test Emergency Override Functions**
```bash
# Emergency override to specific tier (owner only)
registry.emergency_tier_override(token_id, Tier::Tier2, "Testing emergency override")
# Expected: Immediate tier change, bypasses any grace period

# Check immediate effect
registry.get_enhanced_token_data(token_id)
# Expected: tier should be Tier2, no pending changes

# Check events
# Expected: EmergencyTierOverride event with reason

# Emergency override to calculated tier
registry.emergency_tier_override_to_calculated(token_id, "Oracle data updated")
# Expected: Calculates tier from current market data and applies immediately

# Test from non-owner account
registry.emergency_tier_override(token_id, Tier::Tier1, "Should fail")  # From non-owner
# Expected: Error (Unauthorized)

# Clear pending tier changes
registry.clear_pending_tier_change(token_id)
# Expected: Removes any pending tier changes
```

**Purpose**: Test emergency override capabilities and access control

---

## **Phase 4: 80% Rule & Automatic Tier Shifting**

### 10. **Test 80% Rule Detection**
```bash
# Setup: Add enough tokens to same higher tier (≥80% of total)
# You need at least 5 tokens total for MIN_TOKENS_FOR_TIER_SHIFT

# Check if tier shift should occur
registry.should_shift_tier()
# Expected: Returns higher tier if ≥80% qualify, None otherwise

# Verify calculation
registry.get_tier_distribution()
# Expected: Manual verification of 80% threshold
```

**Purpose**: Test automatic tier shift detection logic

### 11. **Trigger Automatic Tier Shift**
```bash
# Method 1: Add tokens until 80% qualify for higher tier
# Continue adding high-tier tokens...

# Method 2: Manual override (owner only)
registry.shift_active_tier(new_tier, "manual_override")
# Expected: Success for owner, failure for non-owner

# Verify tier shifted
registry.get_active_tier()
# Expected: Shows new active tier

registry.get_last_tier_change()
# Expected: Shows recent timestamp

# Check events
# Expected: ActiveTierShifted event with trigger_reason
```

**Purpose**: Test both automatic and manual tier shifting

### 12. **Test Batch Operations**
```bash
# Update all token tiers at once
registry.refresh_all_tiers()
# Expected: Returns count of updated tokens

# Check for changes
registry.get_tier_distribution()
# Expected: May show distribution changes

# Monitor events
# Expected: Multiple TokenTierChanged events if tiers updated
```

**Purpose**: Test bulk tier updates based on current market data

---

## **Phase 5: Configuration Management**

### 13. **Update Tier Thresholds**
```bash
# Test invalid thresholds (should fail)
registry.set_tier_thresholds(invalid_thresholds_wrong_order)
# Expected: Error (InvalidParameter)

# Set valid new thresholds (owner only)
new_thresholds = TierThresholds {
    tier1_market_cap_usd: 75_000_000,  // Increased from $50M
    tier1_volume_usd: 7_500_000,       // Increased from $5M
    // ... other tiers
}
registry.set_tier_thresholds(new_thresholds)
# Expected: Success for owner

# Verify update
registry.get_tier_thresholds()
# Expected: Returns new threshold values

# Check events
# Expected: TierThresholdsUpdated event
```

**Purpose**: Test tier threshold configuration management

### **NEW 13b. Test Grace Period Configuration**
```bash
# Test grace period limits
registry.get_grace_period_limits()
# Expected: Returns (MIN_GRACE_PERIOD_MS, MAX_GRACE_PERIOD_MS)
# Expected: (3_600_000, 31_536_000_000) - 1 hour to 365 days

# Test invalid grace periods (should fail)
registry.set_grace_period(1000)  # Too short (< 1 hour)
# Expected: Error (InvalidParameter)

registry.set_grace_period(40_000_000_000_000)  # Too long (> 365 days)
# Expected: Error (InvalidParameter)

# Set valid grace period for testing
registry.set_grace_period(3_600_000)  # 1 hour
# Expected: Success, GracePeriodUpdated event

# Verify grace period settings
registry.get_grace_period()
# Expected: 3_600_000

registry.get_grace_period_hours()
# Expected: 1

registry.get_grace_period_days()
# Expected: 0 (since 1 hour < 1 day)

# Test from non-owner account
registry.set_grace_period(7_200_000)  # From non-owner
# Expected: Error (Unauthorized)

# Reset to longer period for continued testing
registry.set_grace_period(1_800_000)  # 30 minutes
# Expected: Success
```

**Purpose**: Test adjustable grace period configuration and validation

### 14. **Test USD Rate Conversion**
```bash
# Get current conversion rate
registry.get_current_usd_rate()
# Expected: Returns reasonable plancks-per-USD rate
# Example: If DOT=$7, should return ~1.43B plancks per USD
```

**Purpose**: Verify DOT/USD oracle integration works correctly

---

## **Phase 6: Query Functions & Data Integrity**

### 15. **Test Enhanced Query Functions**
```bash
# Get tokens by specific tier
registry.get_tokens_by_tier(Tier::Tier1)
# Expected: Array of token IDs in Tier1

# Get enriched data with live prices
registry.get_token_data(token_id)
# Expected: Returns enriched data with current oracle prices

# Test backward compatibility
registry.get_basic_token_data(token_id)
# Expected: Returns basic TokenData structure

# Lookup by contract address
registry.get_token_id_by_contract(contract_address)
# Expected: Returns corresponding token_id

# NEW: Test grace period query functions
registry.get_grace_period_end_time(token_id)
# Expected: Returns end time if token has pending tier change, None otherwise

registry.get_grace_period_remaining(token_id)
# Expected: Returns remaining time in ms if active grace period, None otherwise

# Test with multiple tokens in different grace period states
for token_id in [1, 2, 3, 4, 5]:
    remaining = registry.get_grace_period_remaining(token_id)
    expired = registry.is_grace_period_expired(token_id)
    # Expected: Different states for different tokens
```

**Purpose**: Validate all query functions work correctly including new grace period queries

### 16. **Test Tier Distribution Accuracy**
```bash
# Manual count: Count tokens in each tier manually from your records
# Then compare with contract

registry.get_tier_distribution()
# Expected: Counts should match your manual count exactly

# NEW: Verify pending changes don't affect current distribution
pending_tokens = registry.get_tokens_with_pending_changes()
# Expected: Shows tokens with pending changes

# Verify current tier distribution only reflects active tiers, not pending
# Manual verification: pending tier changes should not be counted in current distribution
```

**Purpose**: Ensure tier distribution cache is accurate and properly handles pending changes

---

## **Phase 7: Error Handling & Edge Cases**

### 17. **Test Authorization**
```bash
# Test from non-owner account
registry.set_tier_thresholds(any_thresholds)  # From non-owner
# Expected: Error (Unauthorized)

registry.grant_role(Role::TokenManager, some_account)  # From non-owner
# Expected: Error (Unauthorized)

# NEW: Test emergency override authorization
registry.emergency_tier_override(token_id, Tier::Tier1, "Should fail")  # From non-owner
# Expected: Error (Unauthorized)

registry.set_grace_period(3600000)  # From non-owner
# Expected: Error (Unauthorized)

# Test from account without TokenManager role
registry.add_token(token_contract, oracle_contract)  # From non-TokenManager
# Expected: Error (UnauthorizedRole)
```

**Purpose**: Verify role-based access control works correctly including new functions

### 18. **Test Input Validation**
```bash
# Test zero address
registry.add_token(zero_address, oracle_contract)
# Expected: Error (ZeroAddress)

# Test invalid weight
registry.update_token(token_id, balance, 11000)  # Weight > 10000
# Expected: Error (InvalidWeight)

# Test invalid tier thresholds
registry.set_tier_thresholds(descending_order_thresholds)
# Expected: Error (InvalidParameter)

# NEW: Test grace period validation
registry.set_grace_period(1000)  # Too short (< 1 hour)
# Expected: Error (InvalidParameter)

registry.set_grace_period(50_000_000_000_000)  # Too long (> 365 days)
# Expected: Error (InvalidParameter)

# Test emergency override with non-existent token
registry.emergency_tier_override(999, Tier::Tier1, "Non-existent token")
# Expected: Error (TokenNotFound)
```

**Purpose**: Ensure proper input validation and error handling including new parameters

### 19. **Test Oracle Failure Handling**
```bash
# Scenario: Oracle contract unavailable or returns None
# You can test this by setting wrong oracle address temporarily

registry.set_dot_usd_oracle(non_existent_address)
registry.get_current_usd_rate()
# Expected: Returns None, doesn't panic

# Test tier calculation with failed oracle
registry.calculate_token_tier(token_id)
# Expected: Error (OracleCallFailed) or graceful handling

# NEW: Test emergency override when oracle fails
registry.emergency_tier_override_to_calculated(token_id, "Should handle gracefully")
# Expected: Error (OracleCallFailed) when oracle is unavailable

# Reset oracle for continued testing
registry.set_dot_usd_oracle(correct_oracle_address)
```

**Purpose**: Test resilience to external dependency failures including new oracle-dependent functions

---

## **Phase 8: Event Monitoring**

### 20. **Verify All Events**
Monitor blockchain events during testing for:

**Token Management Events:**
- `TokenAdded` - When tokens are registered
- `TokenUpdated` - When token data is modified  
- `TokenRemoved` - When tokens are removed

**Tier System Events:**
- `TokenTierChanged` - When individual token tiers change
- `ActiveTierShifted` - When the overall active tier shifts
- `GracePeriodStarted` - When tier change grace periods begin
- `TierThresholdsUpdated` - When tier thresholds are modified

**NEW: Grace Period & Emergency Events:**
- `GracePeriodUpdated` - When grace period duration is changed
- `EmergencyTierOverride` - When emergency tier overrides are used

**Access Control Events:**
- `RoleGranted` - When roles are granted
- `RoleRevoked` - When roles are revoked

**Error Events:**
- `OperationFailed` - When operations fail with details

**Detailed Event Testing:**
```bash
# Test grace period event details
registry.set_grace_period(1800000)  # 30 minutes
# Expected: GracePeriodUpdated event with old_period_ms, new_period_ms, updated_by, timestamp

# Test emergency override event details
registry.emergency_tier_override(token_id, Tier::Tier2, "Market emergency")
# Expected: EmergencyTierOverride event with all fields populated

# Test enhanced TokenTierChanged reasons
# Expected reasons: "automatic", "manual", "grace_period_ended", "emergency_override"

# Monitor event sequence during grace period flow
# 1. GracePeriodStarted (when tier change triggered)
# 2. GracePeriodUpdated (if grace period changed)
# 3. TokenTierChanged with "grace_period_ended" (when processed)
```

**Purpose**: Ensure comprehensive event logging for monitoring and debugging including new events

---

## **Phase 9: Performance & Gas Testing**

### 21. **Test Gas Limits**
```bash
# Test with many tokens (10+ tokens)
registry.refresh_all_tiers()
# Monitor: Gas consumption, check if it completes successfully

# Test with many pending changes
registry.process_grace_periods()
# Monitor: Gas usage and success rate

# Test tier distribution calculation with many tokens
registry.should_shift_tier()
# Monitor: Computational efficiency
```

**Purpose**: Ensure functions scale properly with data size

---

## **Phase 10: Integration Testing**

### 22. **Cross-Contract Calls**
```bash
# Verify oracle integration
registry.get_current_usd_rate()
# Expected: Should return reasonable USD conversion rate

# Test token data enrichment
registry.get_token_data(token_id)
# Expected: Should include live price, market_cap, volume from oracle

# Verify tier calculations use live data
registry.calculate_token_tier(token_id)
# Expected: Should reflect current market conditions from oracle
```

**Purpose**: Test complete integration with oracle contracts

---

## **Testing Tips & Best Practices**

### **1. Event Monitoring**
- Always check events after state-changing operations
- Events provide detailed information about what happened
- Use events to verify operations completed correctly

### **2. State Verification**
- Always verify state changes with query functions
- Cross-check calculated values manually
- Ensure consistency between different query methods

### **3. Boundary Testing**
- Test exactly at 80% threshold scenarios
- Test with minimum token counts (5 tokens for tier shifts)
- Test with maximum values and edge cases

### **4. Mock Data Strategy**
- Use test oracle contracts with controllable data
- Set known price/market cap values for predictable results
- Test both positive and negative tier changes

### **5. Time-Based Testing**
- For grace period testing, consider using test environment with controllable time
- Test both before and after grace period expiration
- Verify timestamp handling across different time zones

### **6. Multi-Account Testing**
- Test role-based access from different accounts
- Verify owner vs non-owner permissions
- Test unauthorized access attempts

### **7. Error Case Coverage**
- Test zero values, maximum values, and invalid inputs
- Test with non-existent token IDs
- Test with invalid contract addresses

### **8. Integration Scenarios**
- Test complete workflows from token addition to tier shifting
- Test recovery from oracle failures
- Test system behavior under various market conditions

---

## **Success Criteria**

✅ **All 22 phases completed without critical errors**  
✅ **Event logging comprehensive and accurate (including new grace period events)**  
✅ **Role-based access control enforced correctly (including emergency functions)**  
✅ **Tier calculations reflect real market data**  
✅ **80% rule triggers automatic tier shifts**  
✅ **Grace periods handled correctly with adjustable timing**  
✅ **NEW: Grace period monitoring functions work accurately**  
✅ **NEW: Emergency override functions bypass grace periods correctly**  
✅ **NEW: Grace period configuration validated and secure**  
✅ **USD-based thresholds work with oracle data**  
✅ **Error handling graceful and informative**  
✅ **Performance acceptable with expected data loads**  
✅ **Cross-contract integration reliable**

## **New Testing Commands Summary**

### **Grace Period Management:**
```bash
# Configuration
registry.set_grace_period(period_ms)          # Set grace period (owner only)
registry.get_grace_period()                   # Get in milliseconds
registry.get_grace_period_days()              # Get in days
registry.get_grace_period_hours()             # Get in hours
registry.get_grace_period_limits()            # Get min/max limits

# Monitoring
registry.get_grace_period_end_time(token_id)      # When grace period ends
registry.get_grace_period_remaining(token_id)     # Time remaining
registry.is_grace_period_expired(token_id)        # Has it expired?
```

### **Emergency Override:**
```bash
# Override Functions
registry.emergency_tier_override(token_id, tier, reason)           # Override to specific tier
registry.emergency_tier_override_to_calculated(token_id, reason)   # Override to calculated tier
registry.clear_pending_tier_change(token_id)                      # Clear pending changes
```

### **Enhanced Events to Monitor:**
- `GracePeriodUpdated` - Grace period duration changes
- `EmergencyTierOverride` - Emergency tier override actions
- `TokenTierChanged` - Now includes "emergency_override" reason

This comprehensive testing validates the complete enhanced tier system functionality with adjustable grace periods and emergency controls, ensuring production readiness with flexible management capabilities.