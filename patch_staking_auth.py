import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# ── 1) /api/staking/stake ──
old_stake = '''"/api/staking/stake" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct StakeInput {
            address: String,
            amount: u128,
        }
        match serde_json::from_str::<StakeInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                let mut staking = std::mem::take(&mut chain.staking);
                let result = staking.stake(&mut *chain, &input.address, input.amount);
                chain.staking = staking;
                match result {
                    Ok(total) => json_response(200, json!({
                        "status": "ok",
                        "address": input.address,
                        "staked_this_call": input.amount.to_string(),
                        "total_bonded_stake": total.to_string()
                    }), &cors_origin),
                    Err(e) => bad_request(&format!("Stake failed: {}", e), &cors_origin)
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}'''

new_stake = '''"/api/staking/stake" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct StakeInput {
            address: String,
            amount: u128,
            signature: String,
            timestamp: u64,
        }
        match serde_json::from_str::<StakeInput>(&body_string) {
            Ok(input) => {
                // ── [SECURITY FIX] Signature-authorized stake ──
                // Previously this endpoint accepted a plain address with
                // no proof of ownership, letting anyone stake on behalf
                // of any address (since the API key is effectively
                // public). Now requires a fresh personal_sign signature,
                // same pattern as /swap and /usdt_withdraw.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let auth_ok = if input.timestamp == 0 || now.saturating_sub(input.timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_STAKE:{}:{}:{}",
                        input.address, input.amount, input.timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &input.signature, &input.address)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else {
                    let mut chain = blockchain.lock().expect("chain lock");
                    let mut staking = std::mem::take(&mut chain.staking);
                    let result = staking.stake(&mut *chain, &input.address, input.amount);
                    chain.staking = staking;
                    match result {
                        Ok(total) => json_response(200, json!({
                            "status": "ok",
                            "address": input.address,
                            "staked_this_call": input.amount.to_string(),
                            "total_bonded_stake": total.to_string()
                        }), &cors_origin),
                        Err(e) => bad_request(&format!("Stake failed: {}", e), &cors_origin)
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}'''

if content.count(old_stake) != 1:
    print(f"ERROR: stake anchor found {content.count(old_stake)} times, expected 1")
    sys.exit(1)
content = content.replace(old_stake, new_stake)
print("Patched: /api/staking/stake")

# ── 2) /api/staking/unstake ──
old_unstake = '''"/api/staking/unstake" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct UnstakeInput {
            address: String,
            amount: u128,
        }
        match serde_json::from_str::<UnstakeInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                match chain.staking.unstake(&input.address, input.amount) {
                    Ok(unbonding_id) => json_response(200, json!({
                        "status": "ok",
                        "address": input.address,
                        "unbonding_amount": input.amount.to_string(),
                        "unbonding_entry_id": unbonding_id.to_string(),
                        "unlocks_after_secs": crate::staking::UNBONDING_PERIOD_SECS.to_string()
                    }), &cors_origin),
                    Err(e) => bad_request(&format!("Unstake failed: {}", e), &cors_origin)
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}'''

new_unstake = '''"/api/staking/unstake" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct UnstakeInput {
            address: String,
            amount: u128,
            signature: String,
            timestamp: u64,
        }
        match serde_json::from_str::<UnstakeInput>(&body_string) {
            Ok(input) => {
                // ── [SECURITY FIX] Signature-authorized unstake ──
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let auth_ok = if input.timestamp == 0 || now.saturating_sub(input.timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_UNSTAKE:{}:{}:{}",
                        input.address, input.amount, input.timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &input.signature, &input.address)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else {
                    let mut chain = blockchain.lock().expect("chain lock");
                    match chain.staking.unstake(&input.address, input.amount) {
                        Ok(unbonding_id) => json_response(200, json!({
                            "status": "ok",
                            "address": input.address,
                            "unbonding_amount": input.amount.to_string(),
                            "unbonding_entry_id": unbonding_id.to_string(),
                            "unlocks_after_secs": crate::staking::UNBONDING_PERIOD_SECS.to_string()
                        }), &cors_origin),
                        Err(e) => bad_request(&format!("Unstake failed: {}", e), &cors_origin)
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}'''

if content.count(old_unstake) != 1:
    print(f"ERROR: unstake anchor found {content.count(old_unstake)} times, expected 1")
    sys.exit(1)
content = content.replace(old_unstake, new_unstake)
print("Patched: /api/staking/unstake")

# ── 3) /api/staking/claim ──
old_claim = '''"/api/staking/claim" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct ClaimInput {
            address: String,
        }
        match serde_json::from_str::<ClaimInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                let mut staking = std::mem::take(&mut chain.staking);
                let result = staking.claim_unbonded(&mut *chain, &input.address);
                chain.staking = staking;
                match result {
                    Ok(total) => json_response(200, json!({
                        "status": "ok",
                        "address": input.address,
                        "claimed_amount": total.to_string()
                    }), &cors_origin),
                    Err(e) => bad_request(&format!("Claim failed: {}", e), &cors_origin)
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}'''

new_claim = '''"/api/staking/claim" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct ClaimInput {
            address: String,
            signature: String,
            timestamp: u64,
        }
        match serde_json::from_str::<ClaimInput>(&body_string) {
            Ok(input) => {
                // ── [SECURITY FIX] Signature-authorized claim ──
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let auth_ok = if input.timestamp == 0 || now.saturating_sub(input.timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_CLAIM:{}:{}",
                        input.address, input.timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &input.signature, &input.address)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else {
                    let mut chain = blockchain.lock().expect("chain lock");
                    let mut staking = std::mem::take(&mut chain.staking);
                    let result = staking.claim_unbonded(&mut *chain, &input.address);
                    chain.staking = staking;
                    match result {
                        Ok(total) => json_response(200, json!({
                            "status": "ok",
                            "address": input.address,
                            "claimed_amount": total.to_string()
                        }), &cors_origin),
                        Err(e) => bad_request(&format!("Claim failed: {}", e), &cors_origin)
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}'''

if content.count(old_claim) != 1:
    print(f"ERROR: claim anchor found {content.count(old_claim)} times, expected 1")
    sys.exit(1)
content = content.replace(old_claim, new_claim)
print("Patched: /api/staking/claim")

with open(path, "w") as f:
    f.write(content)

print("Done.")
