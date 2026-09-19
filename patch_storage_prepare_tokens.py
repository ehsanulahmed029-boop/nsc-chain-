import shutil, datetime, sys

path = "/root/nsc-chain/src/storage.rs"
backup = f"/root/backups/storage.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# ── 1) save_tokens -> prepare_tokens_write() + thin wrapper ──
old1 = '''pub fn save_tokens(tokens: &Vec<crate::token::Token>) {
    // [FIX-11] Same hardening: atomic_write + data_dir().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    let path = data_dir().join("tokens.json");
    // total_supply and balances are u128 (18-decimal scaled) and can
    // exceed what JSON numbers safely hold; serialize as strings.
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
    match serde_json::to_string_pretty(&list) {
        Ok(s) => {
            if let Err(e) = atomic_write(&path, s.as_bytes()) {
                eprintln!("[STORAGE] Failed to save tokens: {}", e);
            }
        }
        Err(e) => eprintln!("[STORAGE] Failed to serialise tokens: {}", e),
    }
}'''

new1 = '''/// [ATOMICITY] Same purpose as prepare_pool_write() but for
/// tokens.json (the full custom-token registry, including all
/// per-holder balances). Callers that mutate a token's balances as
/// part of a larger operation (e.g. a /swap hop) can fold this in
/// with pool/LP writes into one atomic_write_batch().
pub fn prepare_tokens_write(tokens: &Vec<crate::token::Token>) -> Option<(PathBuf, Vec<u8>)> {
    let path = data_dir().join("tokens.json");
    // total_supply and balances are u128 (18-decimal scaled) and can
    // exceed what JSON numbers safely hold; serialize as strings.
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
    match serde_json::to_string_pretty(&list) {
        Ok(s) => Some((path, s.into_bytes())),
        Err(e) => {
            eprintln!("[STORAGE] Failed to serialise tokens: {}", e);
            None
        }
    }
}

pub fn save_tokens(tokens: &Vec<crate::token::Token>) {
    // [FIX-11] Same hardening: atomic_write + data_dir().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    if let Some((path, bytes)) = prepare_tokens_write(tokens) {
        if let Err(e) = atomic_write(&path, &bytes) {
            eprintln!("[STORAGE] Failed to save tokens: {}", e);
        }
    }
}'''

if content.count(old1) != 1:
    print(f"ERROR: save_tokens anchor found {content.count(old1)} times, expected 1")
    sys.exit(1)
content = content.replace(old1, new1)
print("Patched: added prepare_tokens_write(), save_tokens() now uses it")

# ── 2) save_token_lp -> prepare_token_lp_write() + thin wrapper ──
old2 = '''pub fn save_token_lp(symbol: &str, total_lp: u128, holders: &std::collections::HashMap<String, u128>) {
    // [FIX-11] Same hardening: atomic_write + data_dir().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
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

    match serde_json::to_string_pretty(&data) {
        Ok(s) => {
            if let Err(e) = atomic_write(&path, s.as_bytes()) {
                eprintln!("[STORAGE] Failed to save token LP: {}", e);
            }
        }
        Err(e) => eprintln!("[STORAGE] Failed to serialise token LP: {}", e),
    }
}'''

new2 = '''/// [ATOMICITY] Same purpose as prepare_pool_write() but for a single
/// token symbol's entry inside the shared token_lp.json map.
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

    match serde_json::to_string_pretty(&data) {
        Ok(s) => Some((path, s.into_bytes())),
        Err(e) => {
            eprintln!("[STORAGE] Failed to serialise token LP: {}", e);
            None
        }
    }
}

pub fn save_token_lp(symbol: &str, total_lp: u128, holders: &std::collections::HashMap<String, u128>) {
    // [FIX-11] Same hardening: atomic_write + data_dir().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    if let Some((path, bytes)) = prepare_token_lp_write(symbol, total_lp, holders) {
        if let Err(e) = atomic_write(&path, &bytes) {
            eprintln!("[STORAGE] Failed to save token LP: {}", e);
        }
    }
}'''

if content.count(old2) != 1:
    print(f"ERROR: save_token_lp anchor found {content.count(old2)} times, expected 1")
    sys.exit(1)
content = content.replace(old2, new2)
print("Patched: added prepare_token_lp_write(), save_token_lp() now uses it")

# ── 3) save_token_pool -> prepare_token_pool_write() + thin wrapper ──
old3 = '''pub fn save_token_pool(symbol: &str, token_amount: u128, usdt_amount: f64) {
    // [FIX-11] Same hardening: atomic_write + data_dir().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    let path = data_dir().join("token_pools.json");
    let mut pools: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let usdt_micro = (usdt_amount * 1_000_000.0).round() as u64;
    // token_amount is u128 (18-decimal scaled); store as a string.
    pools[symbol] = serde_json::json!({"token": token_amount.to_string(), "usdt_micro": usdt_micro});
    match serde_json::to_string_pretty(&pools) {
        Ok(s) => {
            if let Err(e) = atomic_write(&path, s.as_bytes()) {
                eprintln!("[STORAGE] Failed to save token pool: {}", e);
            }
        }
        Err(e) => eprintln!("[STORAGE] Failed to serialise token pool: {}", e),
    }
}'''

new3 = '''/// [ATOMICITY] Same purpose as prepare_pool_write() but for a single
/// token symbol's entry inside the shared token_pools.json map.
pub fn prepare_token_pool_write(symbol: &str, token_amount: u128, usdt_amount: f64) -> Option<(PathBuf, Vec<u8>)> {
    let path = data_dir().join("token_pools.json");
    let mut pools: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let usdt_micro = (usdt_amount * 1_000_000.0).round() as u64;
    // token_amount is u128 (18-decimal scaled); store as a string.
    pools[symbol] = serde_json::json!({"token": token_amount.to_string(), "usdt_micro": usdt_micro});
    match serde_json::to_string_pretty(&pools) {
        Ok(s) => Some((path, s.into_bytes())),
        Err(e) => {
            eprintln!("[STORAGE] Failed to serialise token pool: {}", e);
            None
        }
    }
}

pub fn save_token_pool(symbol: &str, token_amount: u128, usdt_amount: f64) {
    // [FIX-11] Same hardening: atomic_write + data_dir().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    if let Some((path, bytes)) = prepare_token_pool_write(symbol, token_amount, usdt_amount) {
        if let Err(e) = atomic_write(&path, &bytes) {
            eprintln!("[STORAGE] Failed to save token pool: {}", e);
        }
    }
}'''

if content.count(old3) != 1:
    print(f"ERROR: save_token_pool anchor found {content.count(old3)} times, expected 1")
    sys.exit(1)
content = content.replace(old3, new3)
print("Patched: added prepare_token_pool_write(), save_token_pool() now uses it")

with open(path, "w") as f:
    f.write(content)

print("Done.")
