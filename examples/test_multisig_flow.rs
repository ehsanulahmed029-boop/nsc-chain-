// Standalone, self-contained test of the treasury multisig flow.
// Uses a FRESH throwaway wallet generated here — does NOT touch
// or require any of the real production owner keys.
//
// Verifies:
//   1. A request signed by a non-owner is rejected.
//   2. A request signed with a forged/invalid signature is rejected.
//   3. A request signed correctly by enough real owners is approved.
//   4. execute_spend() succeeds only once approved, and rejects replay.

use nsc_chain::wallet::Wallet;
use nsc_chain::multisig::{TreasuryMultiSig, TreasurySpendRequest, MultiSigError};
use nsc_chain::treasury::Treasury;

fn main() {
    println!("=== Multisig flow self-test (throwaway wallets only) ===\n");

    // Generate 2 throwaway "owner" wallets + 1 outsider wallet.
    let owner_a = Wallet::new();
    let owner_b = Wallet::new();
    let outsider = Wallet::new();

    println!("Test owner A: {}", owner_a.address);
    println!("Test owner B: {}", owner_b.address);
    println!("Outsider    : {}\n", outsider.address);

    let mut multisig = TreasuryMultiSig::new(
        vec![owner_a.address.clone(), owner_b.address.clone()],
        2, // require both
    );

    let mut treasury = Treasury::new();
    treasury.deposit(1000).unwrap();

    let request = TreasurySpendRequest::new(
        "test-req-1".to_string(),
        100,
        "NSCsomeRecipientAddr".to_string(),
        "test spend".to_string(),
    );
    let message = request.canonical_message();

    // ── Test 1: non-owner signature must be rejected ──
    let outsider_sig = outsider.sign(&message);
    let result = multisig.sign(&request, &outsider.address, &outsider_sig, &outsider.public_key_hex());
    match result {
        Err(MultiSigError::NotAnOwner) => println!("[PASS] Non-owner correctly rejected."),
        other => println!("[FAIL] Expected NotAnOwner, got {:?}", other),
    }

    // ── Test 2: forged signature (wrong signer's sig used) must be rejected ──
    let wrong_sig = outsider.sign(&message); // signed by outsider, claimed as owner_a
    let result = multisig.sign(&request, &owner_a.address, &wrong_sig, &owner_a.public_key_hex());
    match result {
        Err(MultiSigError::InvalidSignature) => println!("[PASS] Forged signature correctly rejected."),
        other => println!("[FAIL] Expected InvalidSignature, got {:?}", other),
    }

    // ── Test 3: real signature from owner_a should succeed ──
    let sig_a = owner_a.sign(&message);
    let result = multisig.sign(&request, &owner_a.address, &sig_a, &owner_a.public_key_hex());
    match result {
        Ok(()) => println!("[PASS] Owner A valid signature accepted."),
        other => println!("[FAIL] Expected Ok, got {:?}", other),
    }

    println!("Approved after 1/2 signatures? {}", multisig.is_approved(&request.id));

    // ── Test 4: real signature from owner_b completes approval ──
    let sig_b = owner_b.sign(&message);
    let result = multisig.sign(&request, &owner_b.address, &sig_b, &owner_b.public_key_hex());
    match result {
        Ok(()) => println!("[PASS] Owner B valid signature accepted."),
        other => println!("[FAIL] Expected Ok, got {:?}", other),
    }

    println!("Approved after 2/2 signatures? {}\n", multisig.is_approved(&request.id));

    // ── Test 5: execute_spend should now succeed ──
    match treasury.execute_spend(&request, &mut multisig) {
        Ok(()) => println!("[PASS] Spend executed. New balance: {}", treasury.balance()),
        Err(e) => println!("[FAIL] Execute failed unexpectedly: {}", e),
    }

    // ── Test 6: replay (executing same request id again) must fail ──
    match treasury.execute_spend(&request, &mut multisig) {
        Err(e) => println!("[PASS] Replay correctly rejected: {}", e),
        Ok(()) => println!("[FAIL] Replay was incorrectly allowed!"),
    }

    println!("\n=== Self-test complete ===");
}
