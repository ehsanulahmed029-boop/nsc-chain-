import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

old = '''                                            {
                                                let mut chain = blockchain.lock().expect("chain lock");
                                                let prev_nsc = chain.get_balance(&wallet);
                                                chain.balances.insert(wallet.clone(), prev_nsc - amt);
                                            }
                                            pool.0 += amt;'''

new = '''                                            let evm_state_writes = {
                                                let mut chain = blockchain.lock().expect("chain lock");
                                                let prev_nsc = chain.get_balance(&wallet);
                                                chain.balances.insert(wallet.clone(), prev_nsc - amt);
                                                crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces)
                                            };
                                            pool.0 += amt;'''

if content.count(old) != 1:
    print(f"ERROR: chain-scope anchor found {content.count(old)} times, expected 1")
    sys.exit(1)
content = content.replace(old, new)
print("Patched: chain balances/nonces captured into evm_state_writes before lock drops")

old2 = '''                                                  let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                                  batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));'''

new2 = '''                                                  let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                                  batch_writes.extend(evm_state_writes);'''

if content.count(old2) != 1:
    print(f"ERROR: batch-extend anchor found {content.count(old2)} times, expected 1")
    sys.exit(1)
content = content.replace(old2, new2)
print("Patched: batch block now uses captured evm_state_writes instead of out-of-scope chain")

with open(path, "w") as f:
    f.write(content)

print("Done.")
