import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

old = '''                let nsc: u128 = (nsc_human * crate::genesis::DECIMALS as f64) as u128;
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
                };'''

new = '''                let nsc: u128 = (nsc_human * crate::genesis::DECIMALS as f64) as u128;
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
                //
                // The signed message uses the RAW (unscaled) amount strings
                // exactly as sent by the frontend, not the derived u128/f64
                // values -- large numbers lose precision going through f64
                // multiplication, which caused signature mismatches for
                // /token/liquidity/add. Using raw strings avoids that class
                // of bug entirely (same approach as /usdt_withdraw).
                let nsc_amount_str = match &body["nsc_amount"] {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => "0".to_string(),
                };
                let usdt_amount_str = match &body["usdt_amount"] {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => "0".to_string(),
                };

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let auth_ok = if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_LIQUIDITY_ADD_MAIN:{}:{}:{}:{}",
                        nsc_amount_str, usdt_amount_str, wallet, timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &signature, &wallet)
                };'''

if content.count(old) != 1:
    print(f"ERROR: anchor found {content.count(old)} times, expected 1")
    sys.exit(1)

content = content.replace(old, new)

with open(path, "w") as f:
    f.write(content)

print("Patched: /liquidity/add signature message now uses raw strings")
