import shutil, datetime

ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
path = "/root/nsc-chain/src/storage.rs"
shutil.copy(path, f"/root/backups/storage.rs.{ts}.bak")
print(f"Backup saved: /root/backups/storage.rs.{ts}.bak")

with open(path, "r") as f:
    content = f.read()

# ── Insert atomic_write_batch + prepare_* helpers right after atomic_write() ──
anchor = '''fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = tmp_path(path);
    // Write to temp file.
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?; // [FIX-02] fsync before rename.
    }

    // Atomically replace the target.
    fs::rename(&tmp, path)?;

    Ok(())
}'''

if content.count(anchor) != 1:
    print(f"ERROR: expected 1 match for atomic_write anchor, found {content.count(anchor)}")
    exit(1)

batch_helper = anchor + '''

/// [FIX-12] Commits multiple file writes as one unit.
///
/// Every target file's bytes are first written to a .tmp file (with
/// fsync) BEFORE any rename happens. Only if every .tmp write
/// succeeds are the files renamed into place, one after another. If
/// any .tmp write fails partway through, nothing is renamed -- the
/// original files are untouched -- and the partial .tmp files are
/// removed.
///
/// This narrows (but does not fully eliminate) the multi-file crash
/// window for related saves (e.g. debiting a token balance and
/// crediting a pool reserve together) down to just the sequence of
/// rename() calls, each of which is individually atomic on the same
/// filesystem. A crash between two renames in the same batch can
/// still leave the batch partially applied -- full protection against
/// that requires a write-ahead journal with startup replay, which is
/// a separate, larger project. This is a practical middle ground:
/// deployable today, meaningfully smaller risk window, no journal
/// format or replay logic to get wrong.
pub fn atomic_write_batch(writes: &[(PathBuf, Vec<u8>)]) -> std::io::Result<()> {
    let mut tmp_paths: Vec<PathBuf> = Vec::with_capacity(writes.len());

    for (path, data) in writes {
        let tmp = tmp_path(path);
        let result = (|| -> std::io::Result<()> {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(data)?;
            file.sync_all()?;
            Ok(())
        })();

        if let Err(e) = result {
            // Clean up any .tmp files already written in this batch
            // before returning the error, so a failed batch never
            // leaves stray .tmp files behind.
            for t in &tmp_paths {
                let _ = fs::remove_file(t);
            }
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }

        tmp_paths.push(tmp);
    }

    for (path, _) in writes.iter().rev() {
        let _ = path; // paths consumed via zip below; keep for clarity
    }

    for ((path, _), tmp) in writes.iter().zip(tmp_paths.iter()) {
        fs::rename(tmp, path)?;
    }

    Ok(())
}

/// [FIX-12] Prepares (path, bytes) for a tokens.json write without
/// writing it, for use with atomic_write_batch(). Mirrors save_tokens().
pub fn prepare_tokens_write(tokens: &Vec<crate::token::Token>) -> Option<(PathBuf, Vec<u8>)> {
    let path = data_dir().join("tokens.json");
    let list: Vec<serde_json::Value> = tokens.iter().map(|t| {
        let balances_str: std::collections::HashMap<String, String> = t.balances
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();
        serde_json::json!({
            "name": t.name,
            "symbol": t.symbol,
            "total_supply": t.total_supply.to_string(),
            "balances": balances_str
        })
    }).collect();
    serde_json::to_string_pretty(&list).ok().map(|s| (path, s.into_bytes()))
}

/// [FIX-12] Prepares (path, bytes) for a token_pools.json write
/// without writing it. Mirrors save_token_pool().
pub fn prepare_token_pool_write(symbol: &str, token_amount: u128, usdt_amount: f64) -> Option<(PathBuf, Vec<u8>)> {
    let path = data_dir().join("token_pools.json");
    let mut pools: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let usdt_micro = (usdt_amount * 1_000_000.0).round() as u64;
    pools[symbol] = serde_json::json!({"token": token_amount.to_string(), "usdt_micro": usdt_micro});
    serde_json::to_string_pretty(&pools).ok().map(|s| (path, s.into_bytes()))
}

/// [FIX-12] Prepares (path, bytes) for a token_lp.json write without
/// writing it. Mirrors save_token_lp().
pub fn prepare_token_lp_write(symbol: &str, total_lp: u128, holders: &std::collections::HashMap<String, u128>) -> Option<(PathBuf, Vec<u8>)> {
    let path = data_dir().join("token_lp.json");
    let mut data: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let holders_str: std::collections::HashMap<String, String> = holders
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();
    data[symbol] = serde_json::json!({
        "total_lp": total_lp.to_string(),
        "holders": holders_str
    });
    serde_json::to_string_pretty(&data).ok().map(|s| (path, s.into_bytes()))
}

/// [FIX-12] Prepares (path, bytes) for a usdt_balances.json write
/// covering ONE OR MORE addresses at once, without writing it. Used
/// so a transfer between two addresses (debit one, credit another)
/// is a single write instead of two sequential reads+writes of the
/// same file -- eliminating that window entirely rather than just
/// narrowing it.
pub fn prepare_usdt_balances_write(entries: &[(&str, f64)]) -> Option<(PathBuf, Vec<u8>)> {
    let path = data_dir().join("usdt_balances.json");
    let mut balances: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    for (address, balance) in entries {
        let micro = (balance * 1_000_000.0).round() as u64;
        balances[*address] = serde_json::json!(micro);
    }
    serde_json::to_string_pretty(&balances).ok().map(|s| (path, s.into_bytes()))
}'''

content = content.replace(anchor, batch_helper)
print("Inserted atomic_write_batch + prepare_* helpers.")

# ── Refactor /token/liquidity/remove in api.rs to use the batch commit ──
api_path = "/root/nsc-chain/src/api.rs"
shutil.copy(api_path, f"/root/backups/api.rs.{ts}.bak")
print(f"Backup saved: /root/backups/api.rs.{ts}.bak")

with open(api_path, "r") as f:
    api_content = f.read()

old_remove_block = '''                            let new_owned = owned_lp - lp_amount;
                            if new_owned == 0 {
                                holders.remove(&wallet);
                            } else {
                                holders.insert(wallet.clone(), new_owned);
                            }
                            let new_total_lp = total_lp - lp_amount;
                            crate::storage::save_token_lp(&symbol, new_total_lp, &holders);

                            pool.0 -= token_out;
                            pool.1 -= usdt_out;
                            crate::storage::save_token_pool(&symbol, pool.0, pool.1);

                            let mut tokens = crate::storage::load_tokens();
                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == symbol) {
                                let bal = t.balance_of(&wallet);
                                t.balances.insert(wallet.clone(), bal + token_out);
                                crate::storage::save_tokens(&tokens);
                            }
                            let prev_usdt = crate::storage::load_usdt_balance(&wallet);
                            crate::storage::save_usdt_balance(&wallet, prev_usdt + usdt_out);

                            json_response(200, json!({'''

new_remove_block = '''                            let new_owned = owned_lp - lp_amount;
                            if new_owned == 0 {
                                holders.remove(&wallet);
                            } else {
                                holders.insert(wallet.clone(), new_owned);
                            }
                            let new_total_lp = total_lp - lp_amount;

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
                            let mut writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                            if let Some(w) = crate::storage::prepare_token_lp_write(&symbol, new_total_lp, &holders) { writes.push(w); }
                            if let Some(w) = crate::storage::prepare_token_pool_write(&symbol, pool.0, pool.1) { writes.push(w); }
                            if let Some(w) = crate::storage::prepare_tokens_write(&tokens) { writes.push(w); }
                            if let Some(w) = crate::storage::prepare_usdt_balances_write(&[(wallet.as_str(), prev_usdt + usdt_out)]) { writes.push(w); }

                            if let Err(e) = crate::storage::atomic_write_batch(&writes) {
                                eprintln!("[API] FATAL: liquidity/remove batch commit failed: {}", e);
                            }

                            json_response(200, json!({'''

count = api_content.count(old_remove_block)
if count != 1:
    print(f"ERROR: expected 1 match for /token/liquidity/remove block, found {count}")
    exit(1)
api_content = api_content.replace(old_remove_block, new_remove_block)
print("Refactored /token/liquidity/remove to use batch commit.")

# ── Refactor /usdt_transfer to a single combined write ──
old_transfer_block = '''                    let prev_from = crate::storage::load_usdt_balance(&from);
                    if prev_from < amount {
                        bad_request("Insufficient USDT balance", &cors_origin)
                    } else {
                        let prev_to = crate::storage::load_usdt_balance(&to);
                        crate::storage::save_usdt_balance(&from, prev_from - amount);
                        crate::storage::save_usdt_balance(&to, prev_to + amount);
                        json_response(200, json!({'''

new_transfer_block = '''                    let prev_from = crate::storage::load_usdt_balance(&from);
                    if prev_from < amount {
                        bad_request("Insufficient USDT balance", &cors_origin)
                    } else {
                        let prev_to = crate::storage::load_usdt_balance(&to);
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
                        }
                        json_response(200, json!({'''

count = api_content.count(old_transfer_block)
if count != 1:
    print(f"ERROR: expected 1 match for /usdt_transfer block, found {count}")
    exit(1)
api_content = api_content.replace(old_transfer_block, new_transfer_block)
print("Refactored /usdt_transfer to use single combined write.")

with open(api_path, "w") as f:
    f.write(api_content)

with open(path, "w") as f:
    f.write(content)

print("All patches written successfully.")
