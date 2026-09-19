#!/usr/bin/env python3
import shutil, sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.step2b.bak'
    shutil.copy(path, backup)
    for old, new in replacements:
        count = content.count(old)
        if count != 1:
            print(f"ABORT: {path} — expected 1 match, found {count} for:\n{old[:120]}...")
            sys.exit(1)
        content = content.replace(old, new)
    with open(path, 'w') as f:
        f.write(content)
    print(f"OK: {path} patched (backup at {backup})")

ROUTES = '''
"/api/emergency/freeze" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct FreezeInput {
            target: String, // "treasury" | "chain" | "all"
            signer: String,
            signature_hex: String,
            public_key_hex: String,
        }
        match serde_json::from_str::<FreezeInput>(&body_string) {
            Ok(input) => {
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
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/emergency/unfreeze/request" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct UnfreezeRequestInput {
            id: String,
            target: String, // "treasury" | "chain" | "all"
        }
        match serde_json::from_str::<UnfreezeRequestInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let request = crate::emergency_freeze_multisig::EmergencyFreezeRequest::new(
                    input.id.clone(),
                    input.target.clone(),
                    "unfreeze".to_string(),
                    timestamp,
                );
                chain.pending_freeze_requests.insert(input.id.clone(), request);
                chain.save();
                json_response(200, json!({
                    "status": "ok",
                    "message": "Unfreeze request created",
                    "request_id": input.id
                }), &cors_origin)
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/emergency/unfreeze/sign" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct EmergencySignInput {
            request_id: String,
            signer: String,
            signature_hex: String,
            public_key_hex: String,
        }
        match serde_json::from_str::<EmergencySignInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                match chain.pending_freeze_requests.get(&input.request_id).cloned() {
                    None => json_response(404, json!({
                        "status": "error",
                        "message": "No pending unfreeze request with that id"
                    }), &cors_origin),
                    Some(request) => {
                        match chain.emergency_freeze_multisig.sign(&request, &input.signer, &input.signature_hex, &input.public_key_hex) {
                            Ok(()) => {
                                let approved = chain.emergency_freeze_multisig.is_approved(&input.request_id);
                                let count = chain.emergency_freeze_multisig.signature_count(&input.request_id);
                                chain.save();
                                json_response(200, json!({
                                    "status": "ok",
                                    "message": "Signature recorded",
                                    "approved": approved,
                                    "signatures": count
                                }), &cors_origin)
                            }
                            Err(e) => bad_request(&format!("Signature rejected: {}", e), &cors_origin)
                        }
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/emergency/unfreeze/execute" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct EmergencyExecuteInput {
            request_id: String,
        }
        match serde_json::from_str::<EmergencyExecuteInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                match chain.pending_freeze_requests.get(&input.request_id).cloned() {
                    None => json_response(404, json!({
                        "status": "error",
                        "message": "No pending unfreeze request with that id"
                    }), &cors_origin),
                    Some(request) => {
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
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/emergency/status" => {
    if method != Method::Get {
        method_not_allowed(&cors_origin)
    } else {
        let chain = blockchain.lock().expect("chain lock");
        let pending: Vec<serde_json::Value> = chain.pending_freeze_requests.values().map(|r| {
            json!({
                "id": r.id,
                "target": r.target,
                "action": r.action,
                "signatures": chain.emergency_freeze_multisig.signature_count(&r.id),
                "approved": chain.emergency_freeze_multisig.is_approved(&r.id)
            })
        }).collect();
        json_response(200, json!({
            "status": "ok",
            "chain_frozen": chain.chain_frozen,
            "treasury_frozen": chain.treasury.is_frozen(),
            "pending_unfreeze_requests": pending
        }), &cors_origin)
    }
}

"/transfer" => {'''

patch_file('src/api.rs', [
    ('\n"/transfer" => {', ROUTES)
])

print("\nStep 2b (API routes) patched.")
print("Now run:")
print("  cargo build --release 2>&1 | tail -100")
