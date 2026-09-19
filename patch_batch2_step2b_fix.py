#!/usr/bin/env python3
import shutil, sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.step2b_fix.bak'
    shutil.copy(path, backup)
    for old, new in replacements:
        count = content.count(old)
        if count != 1:
            print(f"ABORT: {path} — expected 1 match, found {count} for:\n{old[:150]}...")
            sys.exit(1)
        content = content.replace(old, new)
    with open(path, 'w') as f:
        f.write(content)
    print(f"OK: {path} patched (backup at {backup})")

# ── Fix /api/emergency/freeze — replace early returns with if/else expression ──
OLD_FREEZE = '''            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                if !chain.emergency_freeze_multisig.owners.iter().any(|o| o == &input.signer) {
                    return bad_request("Signer is not a recognized emergency-freeze owner", &cors_origin);
                }
                let message = format!("NSCEMERGENCYFREEZE:{}", input.target);
                let public_key = match crate::wallet::Wallet::public_key_from_hex(&input.public_key_hex) {
                    Some(pk) => pk,
                    None => return bad_request("Invalid public key hex", &cors_origin),
                };
                if !crate::wallet::Wallet::verify_signature(&public_key, &message, &input.signature_hex) {
                    return bad_request("Signature verification failed", &cors_origin);
                }
                let derived = match crate::wallet::Wallet::address_from_public_key_hex(&input.public_key_hex) {
                    Some(a) => a,
                    None => return bad_request("Could not derive address from public key", &cors_origin),
                };
                if derived != input.signer {
                    return bad_request("Public key does not match claimed signer", &cors_origin);
                }
                match input.target.as_str() {
                    "treasury" => { chain.treasury.freeze(); }
                    "chain" => { chain.chain_frozen = true; }
                    "all" => { chain.treasury.freeze(); chain.chain_frozen = true; }
                    _ => return bad_request("target must be 'treasury', 'chain', or 'all'", &cors_origin),
                }
                chain.save();
                json_response(200, json!({
                    "status": "ok",
                    "message": format!("Emergency freeze activated for target: {}", input.target),
                    "chain_frozen": chain.chain_frozen,
                    "treasury_frozen": chain.treasury.is_frozen()
                }), &cors_origin)
            }'''

NEW_FREEZE = '''            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                if !chain.emergency_freeze_multisig.owners.iter().any(|o| o == &input.signer) {
                    bad_request("Signer is not a recognized emergency-freeze owner", &cors_origin)
                } else {
                    let message = format!("NSCEMERGENCYFREEZE:{}", input.target);
                    let public_key_opt = crate::wallet::Wallet::public_key_from_hex(&input.public_key_hex);
                    if public_key_opt.is_none() {
                        bad_request("Invalid public key hex", &cors_origin)
                    } else {
                        let public_key = public_key_opt.unwrap();
                        if !crate::wallet::Wallet::verify_signature(&public_key, &message, &input.signature_hex) {
                            bad_request("Signature verification failed", &cors_origin)
                        } else {
                            let derived_opt = crate::wallet::Wallet::address_from_public_key_hex(&input.public_key_hex);
                            if derived_opt.is_none() {
                                bad_request("Could not derive address from public key", &cors_origin)
                            } else if derived_opt.unwrap() != input.signer {
                                bad_request("Public key does not match claimed signer", &cors_origin)
                            } else if input.target != "treasury" && input.target != "chain" && input.target != "all" {
                                bad_request("target must be 'treasury', 'chain', or 'all'", &cors_origin)
                            } else {
                                match input.target.as_str() {
                                    "treasury" => { chain.treasury.freeze(); }
                                    "chain" => { chain.chain_frozen = true; }
                                    "all" => { chain.treasury.freeze(); chain.chain_frozen = true; }
                                    _ => unreachable!(),
                                }
                                chain.save();
                                json_response(200, json!({
                                    "status": "ok",
                                    "message": format!("Emergency freeze activated for target: {}", input.target),
                                    "chain_frozen": chain.chain_frozen,
                                    "treasury_frozen": chain.treasury.is_frozen()
                                }), &cors_origin)
                            }
                        }
                    }
                }
            }'''

patch_file('src/api.rs', [(OLD_FREEZE, NEW_FREEZE)])

# ── Fix /api/emergency/unfreeze/execute — replace early returns with if/else ──
OLD_EXECUTE = '''                    Some(request) => {
                        if !chain.emergency_freeze_multisig.is_approved(&input.request_id) {
                            return bad_request("Request has not reached required signature quorum", &cors_origin);
                        }

                        // ── Sanity check 1: checkpoint integrity ──────────
                        if let Some(height) = chain.find_last_valid_checkpoint() {
                            if !chain.verify_checkpoint(height) {
                                return bad_request(
                                    "Aborting unfreeze: last checkpoint failed verification. Manual investigation required.",
                                    &cors_origin
                                );
                            }
                        } else {
                            return bad_request(
                                "Aborting unfreeze: no valid checkpoint found. Manual investigation required.",
                                &cors_origin
                            );
                        }

                        // ── Sanity check 2: balance consistency ───────────
                        let wallet_count_before = chain.balances.len();
                        let total_before: u128 = chain.balances.values().sum();
                        chain.rebuild_balances();
                        chain.apply_full_state();
                        let wallet_count_after = chain.balances.len();
                        let total_after: u128 = chain.balances.values().sum();
                        if wallet_count_before > 0 && wallet_count_after == 0 {
                            return bad_request(
                                "Aborting unfreeze: balance rebuild produced zero wallets from a non-empty state. Manual investigation required.",
                                &cors_origin
                            );
                        }
                        if total_before > 0 && total_after == 0 {
                            return bad_request(
                                "Aborting unfreeze: balance rebuild zeroed total supply. Manual investigation required.",
                                &cors_origin
                            );
                        }

                        // ── All checks passed — execute unfreeze ──────────
                        match request.target.as_str() {
                            "treasury" => { chain.treasury.unfreeze(); }
                            "chain" => { chain.chain_frozen = false; }
                            "all" => { chain.treasury.unfreeze(); chain.chain_frozen = false; }
                            _ => return bad_request("Invalid target on stored request", &cors_origin),
                        }
                        chain.emergency_freeze_multisig.clear(&input.request_id);
                        chain.pending_freeze_requests.remove(&input.request_id);
                        chain.save();
                        json_response(200, json!({
                            "status": "ok",
                            "message": format!("Unfreeze executed for target: {}", request.target),
                            "chain_frozen": chain.chain_frozen,
                            "treasury_frozen": chain.treasury.is_frozen()
                        }), &cors_origin)
                    }'''

NEW_EXECUTE = '''                    Some(request) => {
                        if !chain.emergency_freeze_multisig.is_approved(&input.request_id) {
                            bad_request("Request has not reached required signature quorum", &cors_origin)
                        } else {
                            let checkpoint_ok = match chain.find_last_valid_checkpoint() {
                                Some(height) => chain.verify_checkpoint(height),
                                None => false,
                            };
                            if !checkpoint_ok {
                                bad_request(
                                    "Aborting unfreeze: checkpoint missing or failed verification. Manual investigation required.",
                                    &cors_origin
                                )
                            } else {
                                let wallet_count_before = chain.balances.len();
                                let total_before: u128 = chain.balances.values().sum();
                                chain.rebuild_balances();
                                chain.apply_full_state();
                                let wallet_count_after = chain.balances.len();
                                let total_after: u128 = chain.balances.values().sum();

                                if wallet_count_before > 0 && wallet_count_after == 0 {
                                    bad_request(
                                        "Aborting unfreeze: balance rebuild produced zero wallets from a non-empty state. Manual investigation required.",
                                        &cors_origin
                                    )
                                } else if total_before > 0 && total_after == 0 {
                                    bad_request(
                                        "Aborting unfreeze: balance rebuild zeroed total supply. Manual investigation required.",
                                        &cors_origin
                                    )
                                } else if request.target != "treasury" && request.target != "chain" && request.target != "all" {
                                    bad_request("Invalid target on stored request", &cors_origin)
                                } else {
                                    match request.target.as_str() {
                                        "treasury" => { chain.treasury.unfreeze(); }
                                        "chain" => { chain.chain_frozen = false; }
                                        "all" => { chain.treasury.unfreeze(); chain.chain_frozen = false; }
                                        _ => unreachable!(),
                                    }
                                    chain.emergency_freeze_multisig.clear(&input.request_id);
                                    chain.pending_freeze_requests.remove(&input.request_id);
                                    chain.save();
                                    json_response(200, json!({
                                        "status": "ok",
                                        "message": format!("Unfreeze executed for target: {}", request.target),
                                        "chain_frozen": chain.chain_frozen,
                                        "treasury_frozen": chain.treasury.is_frozen()
                                    }), &cors_origin)
                                }
                            }
                        }
                    }'''

patch_file('src/api.rs', [(OLD_EXECUTE, NEW_EXECUTE)])

print("\nStep 2b fix applied.")
print("Now run:")
print("  cargo build --release 2>&1 | tail -100")
