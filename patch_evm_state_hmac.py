import shutil, datetime, sys

path = "/root/nsc-chain/src/storage.rs"
backup = f"/root/backups/storage.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# ── 1) Add evm_state_hash_path() helper right before save_evm_state ──
old1 = '''/// Saves EVM balances and nonces to disk. Called after every
/// successful eth_sendRawTransaction / nsc_testMint so state
/// survives a node restart.
pub fn save_evm_state(balances: &HashMap<String, u128>, nonces: &HashMap<String, u64>) {'''

new1 = '''/// [INTEGRITY] Sidecar HMAC-SHA256 file for evm_state.json, same
/// pattern as full_state_hash_path(). evm_state.json holds EVM
/// balances/nonces that are merged into chain.balances at startup —
/// without this, an edit to evm_state.json outside of
/// save_evm_state() would bypass the full_state.json integrity
/// check entirely, since the two files are loaded and merged
/// separately.
fn evm_state_hash_path() -> PathBuf {
    data_dir().join("evm_state.hash")
}

/// Saves EVM balances and nonces to disk. Called after every
/// successful eth_sendRawTransaction / nsc_testMint so state
/// survives a node restart.
pub fn save_evm_state(balances: &HashMap<String, u128>, nonces: &HashMap<String, u64>) {'''

if content.count(old1) != 1:
    print(f"ERROR: anchor1 found {content.count(old1)} times, expected 1")
    sys.exit(1)
content = content.replace(old1, new1)
print("Patched: added evm_state_hash_path() helper")

# ── 2) After writing evm_state.json, also write its HMAC ──
old2 = '''    let path = evm_state_path();
    match atomic_write(&path, json.as_bytes()) {
        Ok(_) => {}
        Err(e) => eprintln!("[STORAGE] Failed to save EVM state: {}", e),
    }
}'''

new2 = '''    let path = evm_state_path();
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
            }
        }
        Err(e) => eprintln!("[STORAGE] Failed to save EVM state: {}", e),
    }
}'''

if content.count(old2) != 1:
    print(f"ERROR: anchor2 found {content.count(old2)} times, expected 1")
    sys.exit(1)
content = content.replace(old2, new2)
print("Patched: save_evm_state() now writes integrity hash")

# ── 3) Verify HMAC on load, before parsing balances/nonces ──
old3 = '''    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[STORAGE] Cannot read EVM state file: {}", e);
            return (HashMap::new(), HashMap::new());
        }
    };

    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[STORAGE] Failed to parse EVM state: {}", e);
            return (HashMap::new(), HashMap::new());
        }
    };'''

new3 = '''    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[STORAGE] Cannot read EVM state file: {}", e);
            return (HashMap::new(), HashMap::new());
        }
    };

    // [INTEGRITY] Verify evm_state.json against its sidecar HMAC before
    // trusting its contents, same treatment as full_state.json. A
    // present-but-mismatching hash means the file was changed by
    // something other than save_evm_state() -- treated as fatal, since
    // these balances are merged directly into chain.balances and
    // silently continuing on unverified balance data is worse than
    // refusing to start.
    match compute_state_hmac(content.as_bytes()) {
        Some(computed_hash) => {
            match fs::read_to_string(evm_state_hash_path()) {
                Ok(stored_hash) => {
                    if stored_hash.trim() != computed_hash {
                        eprintln!(
                            "[STORAGE] CRITICAL: evm_state.json FAILED integrity verification. \
                            The file's contents do not match its recorded hash — it may have \
                            been edited outside of normal save operations. Refusing to start. \
                            Investigate /root/nsc-data/evm_state.json and \
                            /root/nsc-data/evm_state.hash manually before proceeding."
                        );
                        std::process::exit(1);
                    }
                    println!("[STORAGE] EVM state integrity verified.");
                }
                Err(_) => {
                    eprintln!(
                        "[STORAGE] WARNING: evm_state.json exists but no integrity hash file \
                        was found at evm_state.hash. If this is the first load since enabling \
                        integrity checking, a hash will be written on the next save and this \
                        warning will stop appearing. If integrity checking was already enabled \
                        before this run, a missing hash file is itself suspicious."
                    );
                }
            }
        }
        None => {
            eprintln!(
                "[STORAGE] WARNING: NSC_STATE_HMAC_KEY not set — skipping EVM state integrity \
                check. Tampering with evm_state.json will not be detected until this key is \
                configured."
            );
        }
    }

    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[STORAGE] Failed to parse EVM state: {}", e);
            return (HashMap::new(), HashMap::new());
        }
    };'''

if content.count(old3) != 1:
    print(f"ERROR: anchor3 found {content.count(old3)} times, expected 1")
    sys.exit(1)
content = content.replace(old3, new3)
print("Patched: load_evm_state() now verifies integrity hash")

with open(path, "w") as f:
    f.write(content)

print("Done.")
