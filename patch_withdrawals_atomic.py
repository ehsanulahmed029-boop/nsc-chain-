import shutil, datetime, sys

path = "/root/nsc-chain/src/storage.rs"
backup = f"/root/backups/storage.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# ── 0) Add Mutex import near top of file, right after the hmac import ──
old0 = "use hmac::{Hmac, Mac};"
new0 = "use hmac::{Hmac, Mac};\nuse std::sync::Mutex;"

if content.count(old0) != 1:
    print(f"ERROR: import anchor found {content.count(old0)} times, expected 1")
    sys.exit(1)
content = content.replace(old0, new0)
print("Patched: added `use std::sync::Mutex;` import")

# ── 1) Add WITHDRAWALS_LOCK + migrate save/load/mark ──
old = '''pub fn save_withdraw_request(nsc_address: &str, bsc_address: &str, amount: f64) {
    let path = "/root/nsc-data/withdrawals.json";
    let mut list: Vec<serde_json::Value> = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    list.push(serde_json::json!({
        "nsc_address": nsc_address,
        "bsc_address": bsc_address,
        "amount": amount,
        "status": "pending",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }));
    if let Ok(s) = serde_json::to_string_pretty(&list) {
        let _ = std::fs::write(path, s);
    }
}

pub fn load_pending_withdrawals() -> Vec<serde_json::Value> {
    let path = "/root/nsc-data/withdrawals.json";
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn mark_withdrawal_done(index: usize) {
    let path = "/root/nsc-data/withdrawals.json";
    let mut list: Vec<serde_json::Value> = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if let Some(item) = list.get_mut(index) {
        item["status"] = serde_json::json!("done");
    }
    if let Ok(s) = serde_json::to_string_pretty(&list) {
        let _ = std::fs::write(path, s);
    }
}'''

new = '''/// [CONCURRENCY FIX] Guards all read-modify-write access to
/// withdrawals.json. Previously save_withdraw_request (called from the
/// API on /usdt_withdraw) and mark_withdrawal_done (called from
/// withdraw-processor.service via /usdt_withdrawal_done) could race:
/// both read the file, both modify their own in-memory copy, and
/// whichever writes last silently discards the other's change. Since
/// mark_withdrawal_done identifies entries by array index, a lost
/// append from save_withdraw_request would also shift indices out from
/// under a processor that already decided which index to mark done.
/// Holding this lock across the whole read-modify-write sequence
/// serialises all access to the file, and atomic_write() (.tmp +
/// fsync + rename) replaces the previous raw fs::write() so a crash
/// mid-write can never leave withdrawals.json truncated or corrupt.
static WITHDRAWALS_LOCK: std::sync::LazyLock<Mutex<()>> = std::sync::LazyLock::new(|| Mutex::new(()));

fn withdrawals_path() -> std::path::PathBuf {
    std::path::PathBuf::from("/root/nsc-data/withdrawals.json")
}

fn read_withdrawals_unlocked() -> Vec<serde_json::Value> {
    std::fs::read_to_string(withdrawals_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_withdrawals_unlocked(list: &[serde_json::Value]) {
    if let Ok(s) = serde_json::to_string_pretty(list) {
        if let Err(e) = atomic_write(&withdrawals_path(), s.as_bytes()) {
            eprintln!("[STORAGE] Failed to save withdrawals.json: {}", e);
        }
    }
}

pub fn save_withdraw_request(nsc_address: &str, bsc_address: &str, amount: f64) {
    let _guard = WITHDRAWALS_LOCK.lock().expect("withdrawals lock");
    let mut list = read_withdrawals_unlocked();
    list.push(serde_json::json!({
        "nsc_address": nsc_address,
        "bsc_address": bsc_address,
        "amount": amount,
        "status": "pending",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }));
    write_withdrawals_unlocked(&list);
}

pub fn load_pending_withdrawals() -> Vec<serde_json::Value> {
    let _guard = WITHDRAWALS_LOCK.lock().expect("withdrawals lock");
    read_withdrawals_unlocked()
}

pub fn mark_withdrawal_done(index: usize) {
    let _guard = WITHDRAWALS_LOCK.lock().expect("withdrawals lock");
    let mut list = read_withdrawals_unlocked();
    if let Some(item) = list.get_mut(index) {
        item["status"] = serde_json::json!("done");
    }
    write_withdrawals_unlocked(&list);
}'''

if content.count(old) != 1:
    print(f"ERROR: main block anchor found {content.count(old)} times, expected 1")
    sys.exit(1)
content = content.replace(old, new)
print("Patched: save_withdraw_request / load_pending_withdrawals / mark_withdrawal_done now use WITHDRAWALS_LOCK + atomic_write")

# ── 2) has_recent_duplicate_withdraw — use the same lock + helper ──
old2 = '''pub fn has_recent_duplicate_withdraw(address: &str, bsc_address: &str, amount: f64) -> bool {
    let path = "/root/nsc-data/withdrawals.json";
    let list: Vec<serde_json::Value> = match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => return false,
    };'''

new2 = '''pub fn has_recent_duplicate_withdraw(address: &str, bsc_address: &str, amount: f64) -> bool {
    let _guard = WITHDRAWALS_LOCK.lock().expect("withdrawals lock");
    let list = read_withdrawals_unlocked();'''

if content.count(old2) != 1:
    print(f"ERROR: duplicate-check anchor found {content.count(old2)} times, expected 1")
    sys.exit(1)
content = content.replace(old2, new2)
print("Patched: has_recent_duplicate_withdraw now uses WITHDRAWALS_LOCK")

with open(path, "w") as f:
    f.write(content)

print("Done.")
