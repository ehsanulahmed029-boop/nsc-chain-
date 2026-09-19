import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# ── NSC -> USDT branch ──
old1 = '''                            if sender_nsc_balance < amt {
                                bad_request("Insufficient NSC balance", &cors_origin)
                            } else {
                                pool.0 += amt;
                                pool.1 -= usdt_out;
                                crate::storage::save_pool(pool.0, pool.1);

                                let mut chain = blockchain.lock().expect("chain lock");
                                let prev_nsc = chain.get_balance(&wallet);
                                chain.balances.insert(wallet.clone(), prev_nsc - amt);
                                crate::storage::save_evm_state(&chain.balances, &chain.nonces);
                                drop(chain);

                                let prev_usdt = crate::storage::load_usdt_balance(&wallet);
                                crate::storage::save_usdt_balance(&wallet, prev_usdt + usdt_out);

                                crate::storage::log_trade("NSC", "USDT", usdt_out, &wallet);'''

new1 = '''                            if sender_nsc_balance < amt {
                                bad_request("Insufficient NSC balance", &cors_origin)
                            } else {
                                pool.0 += amt;
                                pool.1 -= usdt_out;

                                let mut chain = blockchain.lock().expect("chain lock");
                                let prev_nsc = chain.get_balance(&wallet);
                                chain.balances.insert(wallet.clone(), prev_nsc - amt);

                                let prev_usdt = crate::storage::load_usdt_balance(&wallet);

                                // [ATOMICITY FIX] Previously pool.json,
                                // evm_state.json, and usdt_balances.json
                                // were saved as three independent writes.
                                // A crash between any two of them left the
                                // pool debited/credited without the
                                // matching wallet balance change, or vice
                                // versa. Now all three are gathered here
                                // and committed together via
                                // atomic_write_batch(), which only renames
                                // any of them into place after every one's
                                // .tmp write has succeeded.
                                let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {
                                    batch_writes.push(w);
                                }
                                batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));
                                if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt + usdt_out) {
                                    batch_writes.push(w);
                                }
                                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                    eprintln!("[API] Failed to commit swap batch (NSC->USDT): {}", e);
                                }
                                drop(chain);

                                crate::storage::log_trade("NSC", "USDT", usdt_out, &wallet);'''

if content.count(old1) != 1:
    print(f"ERROR: NSC->USDT anchor found {content.count(old1)} times, expected 1")
    sys.exit(1)
content = content.replace(old1, new1)
print("Patched: /swap NSC->USDT branch now uses atomic_write_batch")

# ── USDT -> NSC branch ──
old2 = '''                            if prev_usdt < amt {
                                bad_request("Insufficient USDT balance", &cors_origin)
                            } else {
                                pool.1 += amt;
                                pool.0 -= nsc_out;
                                crate::storage::save_pool(pool.0, pool.1);

                                crate::storage::save_usdt_balance(&wallet, prev_usdt - amt);
                                let mut chain = blockchain.lock().expect("chain lock");
                                let prev_nsc = chain.get_balance(&wallet);
                                chain.balances.insert(wallet.clone(), prev_nsc + nsc_out);
                                crate::storage::save_evm_state(&chain.balances, &chain.nonces);
                                drop(chain);

                                crate::storage::log_trade("USDT", "NSC", amt, &wallet);'''

new2 = '''                            if prev_usdt < amt {
                                bad_request("Insufficient USDT balance", &cors_origin)
                            } else {
                                pool.1 += amt;
                                pool.0 -= nsc_out;

                                let mut chain = blockchain.lock().expect("chain lock");
                                let prev_nsc = chain.get_balance(&wallet);
                                chain.balances.insert(wallet.clone(), prev_nsc + nsc_out);

                                // [ATOMICITY FIX] Same batching as the
                                // NSC->USDT branch above -- pool.json,
                                // evm_state.json, and usdt_balances.json
                                // committed together, all-or-nothing.
                                let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {
                                    batch_writes.push(w);
                                }
                                if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt - amt) {
                                    batch_writes.push(w);
                                }
                                batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));
                                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                    eprintln!("[API] Failed to commit swap batch (USDT->NSC): {}", e);
                                }
                                drop(chain);

                                crate::storage::log_trade("USDT", "NSC", amt, &wallet);'''

if content.count(old2) != 1:
    print(f"ERROR: USDT->NSC anchor found {content.count(old2)} times, expected 1")
    sys.exit(1)
content = content.replace(old2, new2)
print("Patched: /swap USDT->NSC branch now uses atomic_write_batch")

with open(path, "w") as f:
    f.write(content)

print("Done.")
