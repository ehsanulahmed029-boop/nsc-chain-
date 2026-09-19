import shutil, datetime, re

ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
path = "/root/nsc-chain/src/storage.rs"
shutil.copy(path, f"/root/backups/storage.rs.{ts}.bak")
print(f"Backup saved: /root/backups/storage.rs.{ts}.bak")

with open(path, "r") as f:
    content = f.read()

sig = "fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {"
idx = content.find(sig)
if idx == -1:
    print("ERROR: could not find atomic_write signature")
    exit(1)

# find the end of this function: the first "\n}\n" after idx
end_marker = content.find("\n}\n", idx)
if end_marker == -1:
    print("ERROR: could not find end of atomic_write function")
    exit(1)
insert_pos = end_marker + len("\n}\n")

batch_helper = '''
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
/// window for related saves down to just the sequence of rename()
/// calls, each of which is individually atomic on the same
/// filesystem. Full protection against a crash between two renames
/// in the same batch requires a write-ahead journal with startup
/// replay -- a separate, larger project. This is a practical middle
/// ground: deployable today, meaningfully smaller risk window, no
/// journal format or replay logic to get wrong.
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
            for t in &tmp_paths {
                let _ = fs::remove_file(t);
            }
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }

        tmp_paths.push(tmp);
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
/// covering one or more addresses at once, without writing it. Used
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
}
'''

content = content[:insert_pos] + batch_helper + content[insert_pos:]

with open(path, "w") as f:
    f.write(content)

print("Inserted atomic_write_batch + prepare_* helpers into storage.rs successfully.")
