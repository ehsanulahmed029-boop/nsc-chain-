import shutil, datetime, sys

path = "/root/nsc-chain/src/storage.rs"
backup = f"/root/backups/storage.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# ── 1) save_pool -> split into prepare_pool_write() + thin save_pool() ──
old1 = '''pub fn save_pool(nsc: u128, usdt: f64) {
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

new1 = '''/// [ATOMICITY] Builds the (path, bytes) for pool.json without writing
/// it to disk. Lets callers that must commit several related files
/// as a single all-or-nothing unit (e.g. /swap, which touches
/// pool.json + evm_state.json + usdt_balances.json together) gather
/// every write first, then hand them all to atomic_write_batch() in
/// one call instead of saving each file as an independent step.
pub fn prepare_pool_write(nsc: u128, usdt: f64) -> Option<(PathBuf, Vec<u8>)> {
    let path = data_dir().join("pool.json");
    let usdt_micro = (usdt * 1_000_000.0).round() as u64;
    // nsc is u128 (18-decimal) — store as string since JSON numbers
    // can't hold full u128 precision.
    let data = serde_json::json!({"nsc": nsc.to_string(), "usdt_micro": usdt_micro});
    match serde_json::to_string_pretty(&data) {
        Ok(s) => Some((path, s.into_bytes())),
        Err(e) => {
            eprintln!("[STORAGE] Failed to serialise pool: {}", e);
            None
        }
    }
}

pub fn save_pool(nsc: u128, usdt: f64) {
    // [FIX-11] Was a bare std::fs::write() to a hardcoded path — not
    // atomic (a crash mid-write left a truncated/corrupt pool.json)
    // and ignored NSC_DATA_DIR. Now goes through ensure_data_dir() +
    // atomic_write(), same hardened pattern as save_chain().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    if let Some((path, bytes)) = prepare_pool_write(nsc, usdt) {
        if let Err(e) = atomic_write(&path, &bytes) {
            eprintln!("[STORAGE] Failed to save pool: {}", e);
        }
    }
}'''

if content.count(old1) != 1:
    print(f"ERROR: save_pool anchor found {content.count(old1)} times, expected 1")
    sys.exit(1)
content = content.replace(old1, new1)
print("Patched: added prepare_pool_write(), save_pool() now uses it")

# ── 2) save_usdt_balance -> split into prepare_usdt_balance_write() + thin save_usdt_balance() ──
old2 = '''pub fn save_usdt_balance(address: &str, balance: f64) {
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

new2 = '''/// [ATOMICITY] Same purpose as prepare_pool_write() but for a single
/// address's entry inside the shared usdt_balances.json map. Reads
/// the current file to merge the one changed address in (same as
/// save_usdt_balance always did), but returns bytes instead of
/// writing, so it can be folded into a multi-file atomic_write_batch().
pub fn prepare_usdt_balance_write(address: &str, balance: f64) -> Option<(PathBuf, Vec<u8>)> {
    let path = data_dir().join("usdt_balances.json");
    let mut balances: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let micro = (balance * 1_000_000.0).round() as u64;
    balances[address] = serde_json::json!(micro);
    match serde_json::to_string_pretty(&balances) {
        Ok(s) => Some((path, s.into_bytes())),
        Err(e) => {
            eprintln!("[STORAGE] Failed to serialise USDT balances: {}", e);
            None
        }
    }
}

pub fn save_usdt_balance(address: &str, balance: f64) {
    // [FIX-11] Same hardening as save_pool: atomic_write + data_dir().
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }
    if let Some((path, bytes)) = prepare_usdt_balance_write(address, balance) {
        if let Err(e) = atomic_write(&path, &bytes) {
            eprintln!("[STORAGE] Failed to save USDT balance: {}", e);
        }
    }
}'''

if content.count(old2) != 1:
    print(f"ERROR: save_usdt_balance anchor found {content.count(old2)} times, expected 1")
    sys.exit(1)
content = content.replace(old2, new2)
print("Patched: added prepare_usdt_balance_write(), save_usdt_balance() now uses it")

with open(path, "w") as f:
    f.write(content)

print("Done with part 1 (pool + usdt_balance helpers).")
