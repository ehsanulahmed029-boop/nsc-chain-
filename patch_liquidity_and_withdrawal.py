import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# ── Patch 1: /usdt_withdrawal_done — add is_internal_request guard ──
start1 = '"/usdt_withdrawal_done" => {'
end1 = '"/usdt_withdraw" => {'

i1 = content.find(start1)
j1 = content.find(end1)
if i1 == -1 or j1 == -1 or j1 <= i1:
    print("ERROR: could not locate /usdt_withdrawal_done block anchors")
    sys.exit(1)

new_block1 = '''"/usdt_withdrawal_done" => {
    if !is_internal_request(&request) {
        bad_request("This endpoint is not available externally", &cors_origin)
    } else if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let index = body["index"].as_u64().unwrap_or(0) as usize;
                crate::storage::mark_withdrawal_done(index);
                json_response(200, json!({"status": "ok"}), &cors_origin)
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

'''

content = content[:i1] + new_block1 + content[j1:]
print("Patched: /usdt_withdrawal_done")

# ── Patch 2: /liquidity/add — signature verification + wallet debit ──
start2 = '"/liquidity/add" => {'
end2 = '"/token/liquidity/remove" => {'

i2 = content.find(start2)
j2 = content.find(end2)
if i2 == -1 or j2 == -1 or j2 <= i2:
    print("ERROR: could not locate /liquidity/add block anchors")
    sys.exit(1)

new_block2 = '''"/liquidity/add" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                // nsc_amount is a human-entered whole-NSC value (e.g.
                // 1037.5) from the frontend, sent as a JSON number or
                // string. Scale to 18-decimal u128 internal units, same
                // approach as the /swap endpoint.
                let nsc_human: f64 = match &body["nsc_amount"] {
                    serde_json::Value::String(s) => s.parse().unwrap_or(0.0),
                    serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
                    _ => 0.0,
                };
                let nsc: u128 = (nsc_human * crate::genesis::DECIMALS as f64) as u128;
                let usdt = body["usdt_amount"].as_f64().unwrap_or(0.0);
                let wallet = body["wallet"].as_str().unwrap_or("").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                // ── [SECURITY FIX] Signature-authorized liquidity add ──
                // Previously this endpoint added nsc_amount/usdt_amount
                // straight from the request body into the pool with no
                // signature check and no wallet debit, letting anyone
                // inflate the USDT reserve for free and drain real USDT
                // out via /swap + /usdt_withdraw. Now requires a fresh
                // personal_sign signature and debits the wallet's actual
                // NSC and USDT balances, same pattern as /token/liquidity/add.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let auth_ok = if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_LIQUIDITY_ADD_MAIN:{}:{}:{}:{}",
                        nsc, usdt, wallet, timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &signature, &wallet)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else if nsc == 0 || usdt <= 0.0 || wallet.is_empty() {
                    bad_request("Invalid amounts", &cors_origin)
                } else {
                    let sender_nsc_balance = {
                        let chain = blockchain.lock().expect("chain lock");
                        chain.get_balance(&wallet)
                    };
                    let sender_usdt_balance = crate::storage::load_usdt_balance(&wallet);

                    if sender_nsc_balance < nsc {
                        bad_request("Insufficient NSC balance", &cors_origin)
                    } else if sender_usdt_balance < usdt {
                        bad_request("Insufficient USDT balance", &cors_origin)
                    } else {
                        {
                            let mut chain = blockchain.lock().expect("chain lock");
                            let prev_nsc = chain.get_balance(&wallet);
                            chain.balances.insert(wallet.clone(), prev_nsc - nsc);
                            crate::storage::save_evm_state(&chain.balances, &chain.nonces);
                        }
                        crate::storage::save_usdt_balance(&wallet, sender_usdt_balance - usdt);

                        let mut pool = crate::storage::load_pool();
                        pool.0 += nsc;
                        pool.1 += usdt;
                        crate::storage::save_pool(pool.0, pool.1);
                        json_response(200, json!({
                            "status": "ok",
                            "nsc_reserve": pool.0.to_string(),
                            "usdt_reserve": pool.1
                        }), &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

'''

content = content[:i2] + new_block2 + content[j2:]
print("Patched: /liquidity/add")

with open(path, "w") as f:
    f.write(content)

print("Done. Now: touch src/api.rs && cargo build --release")
