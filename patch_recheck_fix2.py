import re, shutil, datetime, os, sys

os.makedirs("/root/backups", exist_ok=True)
ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
PATH = "/root/nsc-chain/src/coingecko_client.rs"
shutil.copy(PATH, f"/root/backups/coingecko_client.rs_pre_recheck_{ts}.bak")

with open(PATH, "r", encoding="utf-8") as f:
    content = f.read()

pattern = r"verified:\s*true,\s*first_seen_at:\s*now_ts\(\),\s*price_usd:\s*0\.0,"
replacement = "verified: true,\n        first_seen_at: now_ts(),\n        last_checked_at: now_ts(),\n        price_usd: 0.0,"

new_content, n = re.subn(pattern, replacement, content, flags=re.DOTALL)
print(f"Replacements made: {n} (expected 2)")

if n != 2:
    print("ABORTING — unexpected match count, file not written")
    sys.exit(1)

with open(PATH, "w", encoding="utf-8") as f:
    f.write(new_content)
print("coingecko_client.rs WRITTEN OK")
