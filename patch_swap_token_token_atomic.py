import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

old = '''                                        } else if !dest_exists {
                                            bad_request("Destination token not found", &cors_origin)
                                        } else {
                                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == from) {
                                                let bal = t.balance_of(&wallet);
                                                t.balances.insert(wallet.clone(), bal - amt);
                                            }
                                            pool_a.0 += amt;
                                            pool_a.1 -= usdt_mid;
                                            crate::storage::save_token_pool(from, pool_a.0, pool_a.1);

                                            pool_b.1 += usdt_mid;
                                            pool_b.0 -= token_out;
                                            crate::storage::enforce_token_floor(to, &mut pool_b);
                                            crate::storage::save_token_pool(to, pool_b.0, pool_b.1);
                                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == to) {
                                                let bal = t.balance_of(&wallet);
                                                t.balances.insert(wallet.clone(), bal + token_out);
                                            }
                                            crate::storage::save_tokens(&tokens);

                                            crate::storage::log_trade(from, to, usdt_mid, &wallet);
                                            json_response(200, json!({
                                                "status": "ok",
                                                "from": from,
                                                "to": to,
                                                "amount_in": amt.to_string(),
                                                "amount_out": token_out.to_string(),
                                                "route": [from, "USDT", to]
                                            }), &cors_origin)
                                        }
                                    }
                            }
                            }
                    } else if from == "NSC" && to != "USDT" && to != "NSC" {'''

new = '''                                        } else if !dest_exists {
                                            bad_request("Destination token not found", &cors_origin)
                                        } else {
                                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == from) {
                                                let bal = t.balance_of(&wallet);
                                                t.balances.insert(wallet.clone(), bal - amt);
                                            }
                                            pool_a.0 += amt;
                                            pool_a.1 -= usdt_mid;

                                            pool_b.1 += usdt_mid;
                                            pool_b.0 -= token_out;
                                            crate::storage::enforce_token_floor(to, &mut pool_b);
                                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == to) {
                                                let bal = t.balance_of(&wallet);
                                                t.balances.insert(wallet.clone(), bal + token_out);
                                            }

                                            // [ATOMICITY FIX] Both token
                                            // pool files and tokens.json
                                            // committed together instead
                                            // of three sequential
                                            // independent saves.
                                            let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                            if let Some(w) = crate::storage::prepare_token_pool_write(from, pool_a.0, pool_a.1) {
                                                batch_writes.push(w);
                                            }
                                            if let Some(w) = crate::storage::prepare_token_pool_write(to, pool_b.0, pool_b.1) {
                                                batch_writes.push(w);
                                            }
                                            if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {
                                                batch_writes.push(w);
                                            }
                                            if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                                eprintln!("[API] Failed to commit swap batch (token->token): {}", e);
                                            }

                                            crate::storage::log_trade(from, to, usdt_mid, &wallet);
                                            json_response(200, json!({
                                                "status": "ok",
                                                "from": from,
                                                "to": to,
                                                "amount_in": amt.to_string(),
                                                "amount_out": token_out.to_string(),
                                                "route": [from, "USDT", to]
                                            }), &cors_origin)
                                        }
                                    }
                            }
                            }
                    } else if from == "NSC" && to != "USDT" && to != "NSC" {'''

if content.count(old) != 1:
    print(f"ERROR: token->token anchor found {content.count(old)} times, expected 1")
    sys.exit(1)
content = content.replace(old, new)
print("Patched: /swap TOKEN_A->TOKEN_B branch now uses atomic_write_batch")

with open(path, "w") as f:
    f.write(content)

print("Done.")
