// Isolated test for chain_frozen enforcement (Batch 2.5, 2026-08-10).
// Run with: NSC_DATA_DIR=/tmp/nsc_test_freeze cargo run --example test_chain_freeze
//
// Verifies:
// 1. mine_pending_transactions() is blocked when chain_frozen = true
// 2. mine_pending_transactions() works normally when chain_frozen = false
// 3. chain_frozen persists correctly through save() / apply_full_state()

use nsc_chain::chain::Blockchain;
use nsc_chain::wallet::Wallet;
use nsc_chain::transaction::Transaction;

fn main() {
    println!("=== Test 1: chain_frozen blocks mining ===\n");

    let mut chain = Blockchain::new();

    // Fresh throwaway sender wallet — not a real production key.
    let sender = Wallet::new();
    let receiver_addr = "NSCtestReceiver000".to_string();

    // Fund the sender directly in in-memory balances (test-only shortcut).
    chain.balances.insert(sender.address.clone(), 1_000_000);

    let amount: u128 = 100;
    let fee: u128 = 1;
    let nonce: u64 = 0;

    let signature = sender.sign_transaction(
        &sender.address,
        &receiver_addr,
        amount,
        fee,
        nonce,
    );

    let tx = Transaction::new(
        sender.address.clone(),
        receiver_addr.clone(),
        amount,
        fee,
        nonce,
        sender.public_key_hex(),
        signature,
    );

    assert!(tx.verify_signature(), "SETUP FAIL: test tx does not verify — check sign/verify wiring");

    chain.mempool.transactions.push(tx);

    chain.chain_frozen = true;
    let height_before = chain.height();
    chain.mine_pending_transactions("NSCtestMiner".to_string());
    let height_after = chain.height();

    assert_eq!(height_before, height_after, "FAIL: block was mined while chain frozen!");
    println!("PASS: no block mined while chain_frozen=true (height stayed {})\n", height_after);

    println!("=== Test 2: unfreezing allows mining again ===\n");
    chain.chain_frozen = false;
    chain.mine_pending_transactions("NSCtestMiner".to_string());
    let height_final = chain.height();
    assert!(height_final > height_after, "FAIL: block was NOT mined after unfreezing!");
    println!("PASS: block mined after chain_frozen=false (height now {})\n", height_final);

    println!("=== Test 3: chain_frozen persists through save/reload ===\n");
    chain.chain_frozen = true;
    chain.save();

    let mut reloaded = Blockchain::new();
    reloaded.apply_full_state();
    assert_eq!(reloaded.chain_frozen, true, "FAIL: chain_frozen did not persist!");
    println!("PASS: chain_frozen=true correctly restored after reload\n");

    println!("=== ALL TESTS PASSED ===");
}
