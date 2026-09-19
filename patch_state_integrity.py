import shutil
import datetime

ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")

def backup(path):
    b = f"/root/backups/{path.split('/')[-1]}.{ts}.bak"
    shutil.copy(path, b)
    print(f"Backup saved: {b}")

def patch(path, edits):
    backup(path)
    with open(path, "r") as f:
        content = f.read()
    for old, new in edits:
        count = content.count(old)
        if count != 1:
            print(f"ERROR in {path}: expected 1 match, found {count} for anchor starting: {old[:60]!r}")
            exit(1)
        content = content.replace(old, new)
    with open(path, "w") as f:
        f.write(content)
    print(f"Patched: {path}")

# ── Cargo.toml: add hmac dependency ──
patch("/root/nsc-chain/Cargo.toml", [
    (
        'sha2 = "0.10"\nserde = { version = "1.0", features = ["derive"] }',
        'sha2 = "0.10"\nhmac = "0.12"\nserde = { version = "1.0", features = ["derive"] }'
    ),
])

# ── storage.rs: imports + helper functions + save/load wiring ──
storage_edits = [
    (
        "use std::fs;\nuse std::io::Write;\nuse std::path::{Path, PathBuf};\n\nuse crate::block::Block;\nuse crate::transaction::Transaction;",
        "use std::fs;\nuse std::io::Write;\nuse std::path::{Path, PathBuf};\n\nuse hmac::{Hmac, Mac};\nuse sha2::Sha256;\n\nuse crate::block::Block;\nuse crate::transaction::Transaction;\n\ntype HmacSha256 = Hmac<Sha256>;"
    ),
    (
        '/// Returns the full path to the full-state file.\nfn full_state_path() -> PathBuf {\n    data_dir().join("full_state.json")\n}',
        '''/// Returns the full path to the full-state file.
fn full_state_path() -> PathBuf {
    data_dir().join("full_state.json")
}

/// Path to the sidecar integrity hash for full_state.json.
///
/// [INTEGRITY] This file holds an HMAC-SHA256 over the exact bytes
/// of full_state.json, keyed by NSC_STATE_HMAC_KEY. Its purpose is
/// to detect any edit to full_state.json made outside of
/// save_full_state() — e.g. a direct file edit by someone with
/// filesystem access — since such an edit will not know the key and
/// cannot produce a matching hash.
fn full_state_hash_path() -> PathBuf {
    data_dir().join("full_state.hash")
}

/// Reads the integrity key from the environment. Returns None (and
/// logs a warning at call sites) if unset, so a missing key disables
/// checking rather than crashing the node — but a PRESENT and
/// MISMATCHING hash is always treated as fatal tampering, never
/// silently ignored.
fn state_hmac_key() -> Option<Vec<u8>> {
    std::env::var("NSC_STATE_HMAC_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .map(|k| k.into_bytes())
}

/// Computes the hex-encoded HMAC-SHA256 of `data` under the
/// configured key. Returns None if no key is configured.
fn compute_state_hmac(data: &[u8]) -> Option<String> {
    let key = state_hmac_key()?;
    let mut mac = HmacSha256::new_from_slice(&key).expect("HMAC accepts key of any length");
    mac.update(data);
    Some(hex::encode(mac.finalize().into_bytes()))
}'''
    ),
    (
        '''    match atomic_write(&path, json.as_bytes()) {
        Ok(_) => println!(
            "[STORAGE] Full state saved ({} balance(s), treasury={}, {} pending spend(s)) to {}.",
            state.balances.len(),
            state.treasury.balance(),
            state.pending_spend_requests.len(),
            path.display()
        ),
        Err(e) => eprintln!("[STORAGE] Failed to save full state: {}", e),
    }
}''',
        '''    match atomic_write(&path, json.as_bytes()) {
        Ok(_) => {
            println!(
                "[STORAGE] Full state saved ({} balance(s), treasury={}, {} pending spend(s)) to {}.",
                state.balances.len(),
                state.treasury.balance(),
                state.pending_spend_requests.len(),
                path.display()
            );
            // [INTEGRITY] Write a fresh HMAC of what was just saved, so
            // any future load can detect if the file was changed by
            // anything other than this function.
            if let Some(hash) = compute_state_hmac(json.as_bytes()) {
                let hash_path = full_state_hash_path();
                if let Err(e) = atomic_write(&hash_path, hash.as_bytes()) {
                    eprintln!("[STORAGE] WARNING: failed to write full state integrity hash: {}", e);
                }
            } else {
                eprintln!("[STORAGE] WARNING: NSC_STATE_HMAC_KEY not set — full state integrity hash NOT written. Tampering with full_state.json will not be detected.");
            }
        }
        Err(e) => eprintln!("[STORAGE] Failed to save full state: {}", e),
    }
}'''
    ),
    (
        '''    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[STORAGE] Failed to read full state file: {}", e);
            return None;
        }
    };

    match serde_json::from_str::<FullState>(&contents) {''',
        '''    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[STORAGE] Failed to read full state file: {}", e);
            return None;
        }
    };

    // [INTEGRITY] Verify full_state.json against its sidecar HMAC
    // before trusting its contents. A present-but-mismatching hash
    // means the file was changed by something other than
    // save_full_state() (e.g. a direct file edit) — this is treated
    // as fatal, since balances are not derivable from block history
    // and silently continuing on unverified balance data is worse
    // than refusing to start.
    match compute_state_hmac(contents.as_bytes()) {
        Some(computed_hash) => {
            match fs::read_to_string(full_state_hash_path()) {
                Ok(stored_hash) => {
                    if stored_hash.trim() != computed_hash {
                        eprintln!(
                            "[STORAGE] CRITICAL: full_state.json FAILED integrity verification. \
                             The file's contents do not match its recorded hash — it may have \
                             been edited outside of normal save operations. Refusing to start. \
                             Investigate /root/nsc-data/full_state.json and \
                             /root/nsc-data/full_state.hash manually before proceeding."
                        );
                        std::process::exit(1);
                    }
                    println!("[STORAGE] Full state integrity verified.");
                }
                Err(_) => {
                    eprintln!(
                        "[STORAGE] WARNING: full_state.json exists but no integrity hash file \
                         was found at full_state.hash. If this is the first load since enabling \
                         integrity checking, a hash will be written on the next save and this \
                         warning will stop appearing. If integrity checking was already enabled \
                         before this run, a missing hash file is itself suspicious."
                    );
                }
            }
        }
        None => {
            eprintln!(
                "[STORAGE] WARNING: NSC_STATE_HMAC_KEY not set — skipping full state integrity \
                 check. Tampering with full_state.json will not be detected until this key is \
                 configured."
            );
        }
    }

    match serde_json::from_str::<FullState>(&contents) {'''
    ),
]
patch("/root/nsc-chain/src/storage.rs", storage_edits)

# ── chain.rs: run audit_balances() automatically after state restore ──
patch("/root/nsc-chain/src/chain.rs", [
    (
        '            self.staking = state.staking;\n            println!("[CHAIN] Full state restored from disk.");\n        } else {',
        '            self.staking = state.staking;\n            println!("[CHAIN] Full state restored from disk.");\n            self.audit_balances();\n        } else {'
    ),
])

print("All patches applied successfully.")
