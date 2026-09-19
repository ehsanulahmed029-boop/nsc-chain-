import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

old = '''                                } else {
                                    crate::storage::save_usdt_balance(&wallet, prev_usdt - amt);
                                    tpool.1 += amt;
                                    tpool.0 -= token_out;
                                    crate::storage::enforce_token_floor(to, &mut tpool);
                                    crate::storage::save_token_pool(to, tpool.0, tpool.1);
                                    let mut tokens = crate::storage::load_tokens();
                                    if let Some(t) = tokens.iter_mut().find(|t| t.symbol == to) {
                                        let bal = t.balance_of(&wallet);
                                        t.balances.insert(wallet.clone(), bal + token_out);
                                        crate::storage::save_tokens(&tokens);
                                        crate::storage::log_trade("USDT", to, amt, &wallet);
                                        json_response(200, json!({
                                            "status": "ok",
                                            "from": "USDT",
                                            "to": to,
                                            "amount_in": amt,
                                            "amount_out": token_out.to_string()
                                        }), &cors_origin)
                                    } else {
                                        bad_request("Token not found", &cors_origin)
                                    }
                                }'''

new = '''                                } else {
                                    tpool.1 += amt;
                                    tpool.0 -= token_out;
                                    crate::storage::enforce_token_floor(to, &mut tpool);
                                    let mut tokens = crate::storage::load_tokens();
                                    if let Some(t) = tokens.iter_mut().find(|t| t.symbol == to) {
                                        let bal = t.balance_of(&wallet);
                                        t.balances.insert(wallet.clone(), bal + token_out);

                                        // [ATOMICITY FIX] usdt_balances.json,
                                        // the token's pool file, and
                                        // tokens.json committed together
                                        // instead of three sequential
                                        // independent saves.
                                        let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                        if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt - amt) {
                                            batch_writes.push(w);
                                        }
                                        if let Some(w) = crate::storage::prepare_token_pool_write(to, tpool.0, tpool.1) {
                                            batch_writes.push(w);
                                        }
                                        if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {
                                            batch_writes.push(w);
                                        }
                                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                            eprintln!("[API] Failed to commit swap batch (USDT->token): {}", e);
                                        }

                                        crate::storage::log_trade("USDT", to, amt, &wallet);
                                        json_response(200, json!({
                                            "status": "ok",
                                            "from": "USDT",
                                            "to": to,
                                            "amount_in": amt,
                                            "amount_out": token_out.to_string()
                                        }), &cors_origin)
                                    } else {
                                        bad_request("Token not found", &cors_origin)
                                    }
                                }'''

if content.count(old) != 1:
    print(f"ERROR: USDT->token anchor found {content.count(old)} times, expected 1")
    sys.exit(1)
content = content.replace(old, new)
print("Patched: /swap USDT->token branch now uses atomic_write_batch")

with open(path, "w") as f:
    f.write(content)

print("Done.")
