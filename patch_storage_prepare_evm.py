import shutil, datetime, sys

path = "/root/nsc-chain/src/storage.rs"
backup = f"/root/backups/storage.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

old = '''pub fn save_evm_state(balances: &HashMap<String, u128>, nonces: &HashMap<String, u64>) {
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }

    // u128 balances can exceed u64::MAX (18-decimal scaled amounts
    // regularly do), which serde_json cannot represent as a JSON
    // number without the arbitrary_precision feature. Store as
    // strings instead, matching the pool.json convention.
    let balances_str: HashMap<String, String> = balances
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();

    let combined = serde_json::json!({
        "balances": balances_str,
        "nonces": nonces,
    });

    let json = match serde_json::to_string_pretty(&combined) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[STORAGE] Failed to serialise EVM state: {}", e);
            return;
        }
    };

    let path = evm_state_path();
    match atomic_write(&path, json.as_bytes()) {
        Ok(_) => {
            // [INTEGRITY] Write a fresh HMAC of what was just saved, so
            // any future load can detect if evm_state.json was changed
            // by anything other than this function.
            if let Some(hash) = compute_state_hmac(json.as_bytes()) {
                let hash_path = evm_state_hash_path();
                if let Err(e) = atomic_write(&hash_path, hash.as_bytes()) {
                    eprintln!("[STORAGE] WARNING: failed to write EVM state integrity hash: {}", e);
                }
            } else {
                eprintln!("[STORAGE] WARNING: NSC_STATE_HMAC_KEY not set — EVM state integrity hash NOT written. Tampering with evm_state.json will not be detected.");
            }'''

new = '''/// [ATOMICITY] Same purpose as prepare_pool_write(), but for
/// evm_state.json + its evm_state.hash integrity sidecar together --
/// both files must land in the same atomic_write_batch() so a crash
/// can never leave one updated and the other stale (which would fail
/// the integrity check on next load even though nothing was actually
/// tampered with).
pub fn prepare_evm_state_write(balances: &HashMap<String, u128>, nonces: &HashMap<String, u64>) -> Vec<(PathBuf, Vec<u8>)> {
    // u128 balances can exceed u64::MAX (18-decimal scaled amounts
    // regularly do), which serde_json cannot represent as a JSON
    // number without the arbitrary_precision feature. Store as
    // strings instead, matching the pool.json convention.
    let balances_str: HashMap<String, String> = balances
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();

    let combined = serde_json::json!({
        "balances": balances_str,
        "nonces": nonces,
    });

    let json = match serde_json::to_string_pretty(&combined) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[STORAGE] Failed to serialise EVM state: {}", e);
            return Vec::new();
        }
    };

    let mut writes = vec![(evm_state_path(), json.clone().into_bytes())];
    if let Some(hash) = compute_state_hmac(json.as_bytes()) {
        writes.push((evm_state_hash_path(), hash.into_bytes()));
    } else {
        eprintln!("[STORAGE] WARNING: NSC_STATE_HMAC_KEY not set — EVM state integrity hash NOT written. Tampering with evm_state.json will not be detected.");
    }
    writes
}

pub fn save_evm_state(balances: &HashMap<String, u128>, nonces: &HashMap<String, u64>) {
    if let Err(e) = ensure_data_dir() {
        eprintln!("[STORAGE] Cannot create data dir: {}", e);
        return;
    }

    let writes = prepare_evm_state_write(balances, nonces);
    if writes.is_empty() {
        return;
    }

    match atomic_write_batch(&writes) {
        Ok(_) => {'''

if content.count(old) != 1:
    print(f"ERROR: anchor found {content.count(old)} times, expected 1")
    sys.exit(1)
content = content.replace(old, new)

with open(path, "w") as f:
    f.write(content)

print("Patched: added prepare_evm_state_write(), save_evm_state() now uses atomic_write_batch()")
