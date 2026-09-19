#!/usr/bin/env python3
"""
Add /api/staking/stake, /api/staking/unstake, /api/staking/claim routes.
Uses std::mem::take() to temporarily move the `staking` field out of
`chain` so `chain` itself can be passed as the &mut BalanceLedger,
avoiding a self-referential mutable borrow (Staking lives inside
Blockchain, but stake()/claim_unbonded() need &mut Blockchain as the
ledger argument at the same time as &mut self on Staking).
"""
import shutil
import sys
from pathlib import Path
from datetime import datetime

ROOT = Path("/root/nsc-chain")
BACKUP_DIR = Path("/root/nsc-chain-archive/staking_api_routes_" +
                   datetime.now().strftime("%Y%m%d_%H%M%S"))
BACKUP_DIR.mkdir(parents=True, exist_ok=True)

def backup(path: Path):
    dest = BACKUP_DIR / path.name
    shutil.copy2(path, dest)
    print(f"  backed up -> {dest}")

def patch_file(path: Path, old: str, new: str, label: str):
    text = path.read_text()
    count = text.count(old)
    if count != 1:
        print(f"ABORT [{label}]: expected 1 match in {path}, found {count}")
        sys.exit(1)
    backup(path)
    text = text.replace(old, new, 1)
    path.write_text(text)
    print(f"  patched [{label}] in {path}")

p = ROOT / "src/api.rs"

old = '"/api/treasury/request-spend" => {'
new = '''"/api/staking/stake" => {
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
}

"/api/staking/unstake" => {
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
}

"/api/staking/claim" => {
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
}

"/api/treasury/request-spend" => {'''

patch_file(p, old, new, "api.rs staking routes")

print("\nPatch applied. Next: cargo build --release")
