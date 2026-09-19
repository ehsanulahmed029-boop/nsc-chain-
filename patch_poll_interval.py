import re, shutil, datetime, os, sys

os.makedirs("/root/backups", exist_ok=True)
ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
PATH = "/root/nsc-chain/src/bsc_watcher.rs"
shutil.copy(PATH, f"/root/backups/bsc_watcher.rs_pre_pollfix_{ts}.bak")

with open(PATH, "r", encoding="utf-8") as f:
    content = f.read()

pattern = r"const POLL_INTERVAL_SECS: u64 = 600;.*"
replacement = "const POLL_INTERVAL_SECS: u64 = 14400; // 4h — reduced from 10min to stay within Moralis free-tier 40,000 CU/day budget (~480 req/day at 84 req/cycle)"

new_content, n = re.subn(pattern, replacement, content)
print(f"Replacements made: {n} (expected 1)")

if n != 1:
    print("ABORTING — unexpected match count, file not written")
    sys.exit(1)

with open(PATH, "w", encoding="utf-8") as f:
    f.write(new_content)
print("bsc_watcher.rs WRITTEN OK")
