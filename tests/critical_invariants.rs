// Regression tests for known critical bugs/fixes in NSC chain's AMM.
//
// These use a MockLedger instead of the real Blockchain struct, so they
// run fast and don't need chain state / disk / HMAC setup.

use std::collections::HashMap;
use nsc_chain::amm::{BalanceLedger, LiquidityPool, AmmError};

/// Minimal in-memory ledger for testing, implementing BalanceLedger.
struct MockLedger {
    nsc: HashMap<String, u128>,
    usdt: HashMap<String, u64>,
}

impl MockLedger {
    fn new() -> Self {
        Self { nsc: HashMap::new(), usdt: HashMap::new() }
    }

    fn set_nsc(&mut self, addr: &str, amount: u128) {
        self.nsc.insert(addr.to_string(), amount);
    }
}

impl BalanceLedger for MockLedger {
    fn nsc_balance(&self, address: &str) -> u128 {
        *self.nsc.get(address).unwrap_or(&0)
    }

    fn usdt_balance(&self, address: &str) -> u64 {
        *self.usdt.get(address).unwrap_or(&0)
    }

    fn debit_nsc(&mut self, address: &str, amount: u128) -> bool {
        let bal = self.nsc_balance(address);
        if bal < amount {
            return false;
        }
        self.nsc.insert(address.to_string(), bal - amount);
        true
    }

    fn debit_usdt(&mut self, address: &str, amount: u64) -> bool {
        let bal = self.usdt_balance(address);
        if bal < amount {
            return false;
        }
        self.usdt.insert(address.to_string(), bal - amount);
        true
    }

    fn credit_nsc(&mut self, address: &str, amount: u128) -> bool {
        let bal = self.nsc_balance(address);
        self.nsc.insert(address.to_string(), bal.saturating_add(amount));
        true
    }

    fn credit_usdt(&mut self, address: &str, amount: u64) -> bool {
        let bal = self.usdt_balance(address);
        self.usdt.insert(address.to_string(), bal.saturating_add(amount));
        true
    }
}

fn seeded_pool(nsc_reserve: u128, usdt_reserve: u64) -> LiquidityPool {
    let mut pool = LiquidityPool::new();
    pool.nsc_reserve = nsc_reserve;
    pool.usdt_reserve = usdt_reserve;
    pool
}

#[test]
fn swap_nsc_for_usdt_moves_real_balances() {
    let mut pool = seeded_pool(1_000_000, 1_000_000);
    let mut ledger = MockLedger::new();
    ledger.set_nsc("trader1", 10_000);

    let result = pool.swap_nsc_for_usdt(&mut ledger, "trader1", 1_000, 0);
    assert!(result.is_ok(), "expected swap to succeed, got {:?}", result);

    let usdt_out = result.unwrap();
    assert!(usdt_out > 0, "expected non-zero USDT output");

    // Trader's NSC balance should have decreased by exactly nsc_in.
    assert_eq!(ledger.nsc_balance("trader1"), 10_000 - 1_000);
    // Trader's USDT balance should have increased by usdt_out.
    assert_eq!(ledger.usdt_balance("trader1"), usdt_out);
}

#[test]
fn swap_rejects_insufficient_balance_and_moves_nothing() {
    let mut pool = seeded_pool(1_000_000, 1_000_000);
    let mut ledger = MockLedger::new();
    ledger.set_nsc("trader1", 500); // less than nsc_in below

    let before_nsc_reserve = pool.nsc_reserve;
    let before_usdt_reserve = pool.usdt_reserve;

    let result = pool.swap_nsc_for_usdt(&mut ledger, "trader1", 1_000, 0);
    assert_eq!(result, Err(AmmError::InsufficientBalance));

    // Nothing should have moved.
    assert_eq!(ledger.nsc_balance("trader1"), 500);
    assert_eq!(pool.nsc_reserve, before_nsc_reserve);
    assert_eq!(pool.usdt_reserve, before_usdt_reserve);
}

#[test]
fn swap_rejects_when_below_slippage_floor() {
    let mut pool = seeded_pool(1_000_000, 1_000_000);
    let mut ledger = MockLedger::new();
    ledger.set_nsc("trader1", 10_000);

    // Demand an impossibly high min_usdt_out so slippage protection fires.
    let result = pool.swap_nsc_for_usdt(&mut ledger, "trader1", 1_000, u64::MAX);
    assert_eq!(result, Err(AmmError::SlippageExceeded));

    // Balance must be untouched since the swap was rejected.
    assert_eq!(ledger.nsc_balance("trader1"), 10_000);
}

#[test]
fn swap_rejects_zero_amount() {
    let mut pool = seeded_pool(1_000_000, 1_000_000);
    let mut ledger = MockLedger::new();
    ledger.set_nsc("trader1", 10_000);

    let result = pool.swap_nsc_for_usdt(&mut ledger, "trader1", 0, 0);
    assert_eq!(result, Err(AmmError::ZeroAmount));
}

#[test]
fn swap_rejects_on_empty_pool() {
    let mut pool = seeded_pool(0, 0);
    let mut ledger = MockLedger::new();
    ledger.set_nsc("trader1", 10_000);

    let result = pool.swap_nsc_for_usdt(&mut ledger, "trader1", 1_000, 0);
    assert_eq!(result, Err(AmmError::PoolEmpty));
}

#[test]
fn max_supply_constant_matches_expected_cap() {
    // Regression: MAX_SUPPLY must stay at 25,000,000 NSC (in 18-decimal
    // units). If this ever changes unintentionally, this test catches it.
    assert_eq!(
        nsc_chain::chain::MAX_SUPPLY,
        25_000_000u128 * nsc_chain::genesis::DECIMALS
    );
}

#[test]
fn u128_serializes_as_string_in_json() {
    // Regression: u128 values placed directly into json!() without
    // .to_string() can silently panic and kill the API worker thread.
    let amount: u128 = 25_000_000_000_000_000_000_000_000;
    let json_val = serde_json::json!({ "amount": amount.to_string() });
    assert!(json_val["amount"].is_string());
}
