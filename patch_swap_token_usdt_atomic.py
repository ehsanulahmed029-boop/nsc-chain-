import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

old = '''                                if let Some(t) = tokens.iter_mut().find(|t| t.symbol == from) {
                                    let bal = t.balance_of(&wallet);
                                    if bal < amt {
                                        bad_request("Insufficient token balance", &cors_origin)
                                    } else {
                                        t.balances.insert(wallet.clone(), bal - amt);
                                        crate::storage::save_tokens(&tokens);
                                        tpool.0 += amt;
                                        tpool.1 -= usdt_out;
                                        crate::storage::save_token_pool(from, tpool.0, tpool.1);
                                        let prev_usdt = crate::storage::load_usdt_balance(&wallet);
                                        crate::storage::save_usdt_balance(&wallet, prev_usdt + usdt_out);
                                        crate::storage::log_trade(from, "USDT", usdt_out, &wallet);
                                        json_response(200, json!({
                                            "status": "ok",
                                            "from": from,
                                            "to": "USDT",
                                            "amount_in": amt.to_string(),
                                            "amount_out": usdt_out
                                        }), &cors_origin)
                                    }
                                } else {
                                    bad_request("Token not found", &cors_origin)
                                }'''

new = '''                                if let Some(t) = tokens.iter_mut().find(|t| t.symbol == from) {
                                    let bal = t.balance_of(&wallet);
                                    if bal < amt {
                                        bad_request("Insufficient token balance", &cors_origin)
                                    } else {
                                        t.balances.insert(wallet.clone(), bal - amt);
                                        tpool.0 += amt;
                                        tpool.1 -= usdt_out;
                                        let prev_usdt = crate::storage::load_usdt_balance(&wallet);

                                        // [ATOMICITY FIX] tokens.json,
                                        // the token's pool file, and
                                        // usdt_balances.json committed
                                        // together instead of three
                                        // sequential independent saves.
                                        let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                        if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {
                                            batch_writes.push(w);
                                        }
                                        if let Some(w) = crate::storage::prepare_token_pool_write(from, tpool.0, tpool.1) {
                                            batch_writes.push(w);
                                        }
                                        if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt + usdt_out) {
                                            batch_writes.push(w);
                                        }
                                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                            eprintln!("[API] Failed to commit swap batch (token->USDT): {}", e);
                                        }

                                        crate::storage::log_trade(from, "USDT", usdt_out, &wallet);
                                        json_response(200, json!({
                                            "status": "ok",
                                            "from": from,
                                            "to": "USDT",
                                            "amount_in": amt.to_string(),
                                            "amount_out": usdt_out
                                        }), &cors_origin)
                                    }
                                } else {
                                    bad_request("Token not found", &cors_origin)
                                }'''

if content.count(old) != 1:
    print(f"ERROR: token->USDT anchor found {content.count(old)} times, expected 1")
    sys.exit(1)
content = content.replace(old, new)
print("Patched: /swap token->USDT branch now uses atomic_write_batch")

with open(path, "w") as f:
    f.write(content)

print("Done.")
