import shutil, datetime, re

ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
path = "/root/nsc-chain/src/api.rs"
shutil.copy(path, f"/root/backups/api.rs.{ts}.bak")
print(f"Backup saved: /root/backups/api.rs.{ts}.bak")

with open(path, "r") as f:
    content = f.read()

# ══════════════════════════════════════════════════════════════
# /token/liquidity/remove
# ══════════════════════════════════════════════════════════════
start_marker = 'crate::storage::save_token_lp(&symbol, new_total_lp, &holders);'
idx1 = content.find(start_marker)
if idx1 == -1:
    print("ERROR: could not find save_token_lp call in /token/liquidity/remove")
    exit(1)

end_marker = 'crate::storage::save_usdt_balance(&wallet, prev_usdt + usdt_out);'
idx2 = content.find(end_marker, idx1)
if idx2 == -1:
    print("ERROR: could not find matching save_usdt_balance call")
    exit(1)
idx2_end = idx2 + len(end_marker)

old_segment = content[idx1:idx2_end]
print("OLD SEGMENT CAPTURED:")
print(repr(old_segment[:200]))

new_segment = '''let new_total_lp = total_lp - lp_amount;

                            pool.0 -= token_out;
                            pool.1 -= usdt_out;

                            let mut tokens = crate::storage::load_tokens();
                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == symbol) {
                                let bal = t.balance_of(&wallet);
                                t.balances.insert(wallet.clone(), bal + token_out);
                            }
                            let prev_usdt = crate::storage::load_usdt_balance(&wallet);

                            // [FIX-12] All four related writes (LP shares,
                            // token pool reserves, token balance, USDT
                            // balance) are committed as one batch rather
                            // than four sequential independent saves, so a
                            // crash partway through cannot leave e.g. the
                            // LP burned but the token/USDT never credited.
                            let mut _batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                            if let Some(w) = crate::storage::prepare_token_lp_write(&symbol, new_total_lp, &holders) { _batch_writes.push(w); }
                            if let Some(w) = crate::storage::prepare_token_pool_write(&symbol, pool.0, pool.1) { _batch_writes.push(w); }
                            if let Some(w) = crate::storage::prepare_tokens_write(&tokens) { _batch_writes.push(w); }
                            if let Some(w) = crate::storage::prepare_usdt_balances_write(&[(wallet.as_str(), prev_usdt + usdt_out)]) { _batch_writes.push(w); }
                            if let Err(e) = crate::storage::atomic_write_batch(&_batch_writes) {
                                eprintln!("[API] FATAL: liquidity/remove batch commit failed: {}", e);
                            }'''

# Now find where the actual old code starts — go back to "let new_total_lp" instead of save_token_lp
back_marker = 'let new_total_lp = total_lp - lp_amount;'
idx0 = content.rfind(back_marker, 0, idx1)
if idx0 == -1:
    print("ERROR: could not find 'let new_total_lp' before save_token_lp")
    exit(1)

old_full_segment = content[idx0:idx2_end]
content = content[:idx0] + new_segment + content[idx2_end:]
print("Replaced /token/liquidity/remove multi-save block with batch commit.")

# ══════════════════════════════════════════════════════════════
# /usdt_transfer
# ══════════════════════════════════════════════════════════════
t_start_marker = 'let prev_to = crate::storage::load_usdt_balance(&to);'
t_idx1 = content.find(t_start_marker)
if t_idx1 == -1:
    print("ERROR: could not find prev_to load in /usdt_transfer")
    exit(1)

t_end_marker = 'crate::storage::save_usdt_balance(&to, prev_to + amount);'
t_idx2 = content.find(t_end_marker, t_idx1)
if t_idx2 == -1:
    print("ERROR: could not find save_usdt_balance(&to...) call")
    exit(1)
t_idx2_end = t_idx2 + len(t_end_marker)

t_new_segment = '''let prev_to = crate::storage::load_usdt_balance(&to);
                        // [FIX-12] Both balance updates go into ONE write
                        // to usdt_balances.json instead of two sequential
                        // read-modify-writes of the same file -- this
                        // fully eliminates (not just narrows) the window
                        // where a crash could leave the sender debited
                        // but the recipient never credited.
                        if let Some((p, bytes)) = crate::storage::prepare_usdt_balances_write(&[
                            (from.as_str(), prev_from - amount),
                            (to.as_str(), prev_to + amount),
                        ]) {
                            if let Err(e) = crate::storage::atomic_write_batch(&[(p, bytes)]) {
                                eprintln!("[API] FATAL: usdt_transfer commit failed: {}", e);
                            }
                        }'''

content = content[:t_idx1] + t_new_segment + content[t_idx2_end:]
print("Replaced /usdt_transfer double-save block with single combined write.")

with open(path, "w") as f:
    f.write(content)

print("All api.rs patches applied successfully.")
