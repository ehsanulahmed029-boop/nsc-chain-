import shutil, datetime

ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
path = "/root/nsc-chain/src/storage.rs"
shutil.copy(path, f"/root/backups/storage.rs.{ts}.bak")
print(f"Backup saved: /root/backups/storage.rs.{ts}.bak")

with open(path, "r") as f:
    content = f.read()

edits = [
    # ── save_pool ──
    (
'''pub fn save_pool(nsc: u128, usdt: f64) {
    let path = "/root/nsc-data/pool.json";
    let usdt_micro = (usdt * 1_000_000.0).round() as u64;
    // nsc is u128 (18-decimal) — store as string since JSON numbers
    // can't hold full u128 precision.
    let data = serde_json::json!({"nsc": nsc.to_string(), "usdt_micro": usdt_micro});
    if let Ok(s) = serde_json::to_string_pretty(&data) {
        let _ = std::fs::write(path, s);
    }
}''',
'''pub fn save_pool(nsc: u128, usdt: f64) {
    // [FIX-11] Was a bare std::fs::write() to a hardcoded path — not
    // atomic (a crash mid-write left a truncated/corrupt pool.json)
    // and ignored NSC_DATA_DIR. Now goes through ensure_data_dir() +
    // atomic_write(), same hardened pattern as save_chain().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    let path = data_dir().join("pool.json");
    let usdt_micro = (usdt * 1_000_000.0).round() as u64;
    // nsc is u128 (18-decimal) — store as string since JSON numbers
    // can't hold full u128 precision.
    let data = serde_json::json!({"nsc": nsc.to_string(), "usdt_micro": usdt_micro});
    match serde_json::to_string_pretty(&data) {
        Ok(s) => {
            if let Err(e) = atomic_write(&path, s.as_bytes()) {
                eprintln!("[STORAGE] Failed to save pool: {}", e);
            }
        }
        Err(e) => eprintln!("[STORAGE] Failed to serialise pool: {}", e),
    }
}'''
    ),
    (
'''pub fn load_pool() -> (u128, f64) {
    let path = "/root/nsc-data/pool.json";''',
'''pub fn load_pool() -> (u128, f64) {
    let path = data_dir().join("pool.json");
    let path = path.as_path();'''
    ),
    # ── save_usdt_balance ──
    (
'''pub fn save_usdt_balance(address: &str, balance: f64) {
    let path = "/root/nsc-data/usdt_balances.json";
    let mut balances: serde_json::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let micro = (balance * 1_000_000.0).round() as u64;
    balances[address] = serde_json::json!(micro);
    if let Ok(s) = serde_json::to_string_pretty(&balances) {
        let _ = std::fs::write(path, s);
    }
}''',
'''pub fn save_usdt_balance(address: &str, balance: f64) {
    // [FIX-11] Same hardening as save_pool: atomic_write + data_dir().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    let path = data_dir().join("usdt_balances.json");
    let mut balances: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let micro = (balance * 1_000_000.0).round() as u64;
    balances[address] = serde_json::json!(micro);
    match serde_json::to_string_pretty(&balances) {
        Ok(s) => {
            if let Err(e) = atomic_write(&path, s.as_bytes()) {
                eprintln!("[STORAGE] Failed to save USDT balance: {}", e);
            }
        }
        Err(e) => eprintln!("[STORAGE] Failed to serialise USDT balances: {}", e),
    }
}'''
    ),
    (
'''pub fn load_usdt_balance(address: &str) -> f64 {
    let path = "/root/nsc-data/usdt_balances.json";
    let micro = std::fs::read_to_string(path)''',
'''pub fn load_usdt_balance(address: &str) -> f64 {
    let path = data_dir().join("usdt_balances.json");
    let micro = std::fs::read_to_string(&path)'''
    ),
    # ── save_tokens ──
    (
'''pub fn save_tokens(tokens: &Vec<crate::token::Token>) {
    let path = "/root/nsc-data/tokens.json";''',
'''pub fn save_tokens(tokens: &Vec<crate::token::Token>) {
    // [FIX-11] Same hardening: atomic_write + data_dir().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    let path = data_dir().join("tokens.json");'''
    ),
    (
'''    if let Ok(s) = serde_json::to_string_pretty(&list) {
        let _ = std::fs::write(path, s);
    }
}

pub fn load_tokens() -> Vec<crate::token::Token> {
    let path = "/root/nsc-data/tokens.json";
    let mut result = Vec::new();
    if let Ok(s) = std::fs::read_to_string(path) {''',
'''    match serde_json::to_string_pretty(&list) {
        Ok(s) => {
            if let Err(e) = atomic_write(&path, s.as_bytes()) {
                eprintln!("[STORAGE] Failed to save tokens: {}", e);
            }
        }
        Err(e) => eprintln!("[STORAGE] Failed to serialise tokens: {}", e),
    }
}

pub fn load_tokens() -> Vec<crate::token::Token> {
    let path = data_dir().join("tokens.json");
    let mut result = Vec::new();
    if let Ok(s) = std::fs::read_to_string(&path) {'''
    ),
    # ── save_token_lp ──
    (
'''pub fn save_token_lp(symbol: &str, total_lp: u128, holders: &std::collections::HashMap<String, u128>) {
    let path = "/root/nsc-data/token_lp.json";
    let mut data: serde_json::Value = std::fs::read_to_string(path)
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

    if let Ok(s) = serde_json::to_string_pretty(&data) {
        let _ = std::fs::write(path, s);
    }
}''',
'''pub fn save_token_lp(symbol: &str, total_lp: u128, holders: &std::collections::HashMap<String, u128>) {
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
    ),
    (
'''pub fn load_token_lp(symbol: &str) -> (u128, std::collections::HashMap<String, u128>) {
    let path = "/root/nsc-data/token_lp.json";
    let data: serde_json::Value = std::fs::read_to_string(path)''',
'''pub fn load_token_lp(symbol: &str) -> (u128, std::collections::HashMap<String, u128>) {
    let path = data_dir().join("token_lp.json");
    let data: serde_json::Value = std::fs::read_to_string(&path)'''
    ),
    # ── save_token_pool ──
    (
'''pub fn save_token_pool(symbol: &str, token_amount: u128, usdt_amount: f64) {
    let path = "/root/nsc-data/token_pools.json";
    let mut pools: serde_json::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let usdt_micro = (usdt_amount * 1_000_000.0).round() as u64;
    // token_amount is u128 (18-decimal scaled); store as a string.
    pools[symbol] = serde_json::json!({"token": token_amount.to_string(), "usdt_micro": usdt_micro});
    if let Ok(s) = serde_json::to_string_pretty(&pools) {
        let _ = std::fs::write(path, s);
    }
}''',
'''pub fn save_token_pool(symbol: &str, token_amount: u128, usdt_amount: f64) {
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
    ),
    (
'''pub fn load_token_pool(symbol: &str) -> (u128, f64) {
    let path = "/root/nsc-data/token_pools.json";
    std::fs::read_to_string(path)''',
'''pub fn load_token_pool(symbol: &str) -> (u128, f64) {
    let path = data_dir().join("token_pools.json");
    std::fs::read_to_string(&path)'''
    ),
]

for old, new in edits:
    count = content.count(old)
    if count != 1:
        print(f"ERROR: expected 1 match, found {count} for anchor starting: {old[:70]!r}")
        exit(1)
    content = content.replace(old, new)

with open(path, "w") as f:
    f.write(content)

print("All edits applied successfully to storage.rs")
