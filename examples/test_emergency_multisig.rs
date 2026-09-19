// Isolated test for EmergencyFreezeMultiSig (Batch 2.5, 2026-08-10).
// Run with: NSC_DATA_DIR=/tmp/nsc_test_emfreeze cargo run --example test_emergency_multisig
//
// Verifies:
// 1. Non-owner signature is rejected.
// 2. Forged/invalid signature is rejected.
// 3. Correct signatures from real owners reach quorum and is_approved() flips true.
// 4. Blockchain persists pending_freeze_requests + emergency_freeze_multisig correctly.

use nsc_chain::wallet::Wallet;
use nsc_chain::emergency_freeze_multisig::{EmergencyFreezeMultiSig, EmergencyFreezeRequest};
use nsc_chain::chain::Blockchain;

fn main() {
    println!("=== Emergency freeze multisig self-test (throwaway wallets only) ===\n");

    let owner_a = Wallet::new();
    let owner_b = Wallet::new();
    let owner_c = Wallet::new();
    let outsider = Wallet::new();

    let mut multisig = EmergencyFreezeMultiSig::new(
        vec![owner_a.address.clone(), owner_b.address.clone(), owner_c.address.clone()],
        3,
    );

    let request = EmergencyFreezeRequest::new(
        "unfreeze-test-1".to_string(),
        "chain".to_string(),
        "unfreeze".to_string(),
        1234567890,
    );

    println!("=== Test 1: non-owner signature rejected ===");
    let msg = request.canonical_message();
    let sig = outsider.sign(&msg);
    let result = multisig.sign(&request, &outsider.address, &sig, &outsider.public_key_hex());
    assert!(result.is_err(), "FAIL: non-owner signature was accepted!");
    println!("PASS: non-owner correctly rejected\n");

    println!("=== Test 2: forged signature rejected ===");
    let fake_sig = "00".repeat(64);
    let result = multisig.sign(&request, &owner_a.address, &fake_sig, &owner_a.public_key_hex());
    assert!(result.is_err(), "FAIL: forged signature was accepted!");
    println!("PASS: forged signature correctly rejected\n");

    println!("=== Test 3: real owner signatures reach quorum ===");
    for owner in [&owner_a, &owner_b, &owner_c] {
        let sig = owner.sign(&msg);
        let result = multisig.sign(&request, &owner.address, &sig, &owner.public_key_hex());
        assert!(result.is_ok(), "FAIL: valid owner signature was rejected: {:?}", result);
        println!("  {} signed ({}/3)", owner.address, multisig.signature_count(&request.id));
    }
    assert!(multisig.is_approved(&request.id), "FAIL: request not approved after 3/3 signatures!");
    println!("PASS: request approved after 3/3 real owner signatures\n");

    println!("=== Test 4: persistence through Blockchain save/reload ===");
    let mut chain = Blockchain::new();
    chain.emergency_freeze_multisig = multisig;
    chain.pending_freeze_requests.insert(request.id.clone(), request.clone());
    chain.save();

    let mut reloaded = Blockchain::new();
    reloaded.apply_full_state();
    assert!(
        reloaded.emergency_freeze_multisig.is_approved(&request.id),
        "FAIL: approved state did not persist!"
    );
    assert!(
        reloaded.pending_freeze_requests.contains_key(&request.id),
        "FAIL: pending request did not persist!"
    );
    println!("PASS: emergency_freeze_multisig and pending_freeze_requests persisted correctly\n");

    println!("=== ALL TESTS PASSED ===");
}
