import re, shutil, datetime, os, sys

os.makedirs("/root/backups", exist_ok=True)
ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")

REG_PATH = "/root/nsc-chain/src/token_registry.rs"
WATCH_PATH = "/root/nsc-chain/src/bsc_watcher.rs"

shutil.copy(REG_PATH, f"/root/backups/token_registry.rs_pre_recheck_{ts}.bak")
shutil.copy(WATCH_PATH, f"/root/backups/bsc_watcher.rs_pre_recheck_{ts}.bak")

def sub_once(content, pattern, repl, label, flags=re.DOTALL):
    new_content, n = re.subn(pattern, repl, content, count=1, flags=flags)
    if n != 1:
        print(f"FAILED (0 matches, aborting this file's changes): {label}")
        return content, False
    print(f"OK: {label}")
    return new_content, True

ok_all = True

# ---------- token_registry.rs ----------
with open(REG_PATH, "r", encoding="utf-8") as f:
    reg = f.read()

reg, ok = sub_once(
    reg,
    r"pub price_usd:\s*f64,\s*\}",
    "pub price_usd: f64,\n    #[serde(default)]\n    pub last_checked_at: u64,\n}",
    "add last_checked_at field to DetectedToken struct"
)
ok_all &= ok

reg, ok = sub_once(
    reg,
    r"first_seen_at:\s*now_ts\(\),\s*price_usd:\s*0\.0,",
    "first_seen_at: now_ts(),\n            last_checked_at: now_ts(),\n            price_usd: 0.0,",
    "set last_checked_at in DetectedToken::unverified()"
)
ok_all &= ok

if ok_all:
    with open(REG_PATH, "w", encoding="utf-8") as f:
        f.write(reg)
    print("token_registry.rs WRITTEN")
else:
    print("token_registry.rs NOT WRITTEN (see failures above)")
    sys.exit(1)

# ---------- bsc_watcher.rs ----------
with open(WATCH_PATH, "r", encoding="utf-8") as f:
    bw = f.read()

bw, ok = sub_once(
    bw,
    r"async fn resolve_token\(",
    (
        "const RECHECK_INTERVAL_SECS: u64 = 21600; // 6h cooldown before re-querying CoinGecko for a still-unverified token\n\n"
        "fn now_ts() -> u64 {\n"
        "    std::time::SystemTime::now()\n"
        "        .duration_since(std::time::UNIX_EPOCH)\n"
        "        .unwrap()\n"
        "        .as_secs()\n"
        "}\n\n"
        "async fn resolve_token("
    ),
    "insert RECHECK_INTERVAL_SECS const + now_ts() helper before resolve_token"
)
ok_all &= ok

bw, ok = sub_once(
    bw,
    r"if let Some\(existing\) = registry\.get_token\(CHAIN_ID, contract_address\)\s*\{\s*return existing;\s*\}",
    (
        "if let Some(existing) = registry.get_token(CHAIN_ID, contract_address) {\n"
        "        if existing.verified {\n"
        "            return existing;\n"
        "        }\n"
        "        let elapsed = now_ts().saturating_sub(existing.last_checked_at);\n"
        "        if elapsed < RECHECK_INTERVAL_SECS {\n"
        "            return existing;\n"
        "        }\n"
        "        // cooldown expired — fall through and retry CoinGecko below\n"
        "    }"
    ),
    "make unverified tokens retry CoinGecko after cooldown instead of being stuck forever"
)
ok_all &= ok

bw, ok = sub_once(
    bw,
    r"if let Some\(found\) = coingecko_client::lookup_token_by_contract\(CHAIN_ID, contract_address\)\.await\s*\{\s*let _ = registry\.insert_token\(found\.clone\(\)\);\s*return found;\s*\}",
    (
        "if let Some(mut found) = coingecko_client::lookup_token_by_contract(CHAIN_ID, contract_address).await {\n"
        "        found.last_checked_at = now_ts();\n"
        "        let _ = registry.insert_token(found.clone());\n"
        "        return found;\n"
        "    }"
    ),
    "stamp last_checked_at when a token is newly verified via CoinGecko"
)
ok_all &= ok

bw, ok = sub_once(
    bw,
    r"price_usd:\s*0\.0,\s*\};\s*let _ = registry\.insert_token\(token\.clone\(\)\);\s*token\s*\}",
    "price_usd: 0.0,\n        last_checked_at: now_ts(),\n    };\n    let _ = registry.insert_token(token.clone());\n    token\n}",
    "stamp last_checked_at on the fallback (still-unverified) token path"
)
ok_all &= ok

if ok_all:
    with open(WATCH_PATH, "w", encoding="utf-8") as f:
        f.write(bw)
    print("bsc_watcher.rs WRITTEN")
    print("ALL PATCHES APPLIED OK")
else:
    print("bsc_watcher.rs NOT WRITTEN (see failures above) — token_registry.rs was already written, restore from backup if needed")
    sys.exit(1)
