import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    lines = f.readlines()

idx = 2345 - 1
if lines[idx].strip() != "}":
    print(f"ERROR: line 2345 is not a bare closing brace. Actual: {lines[idx]!r}")
    sys.exit(1)

lines[idx] = lines[idx].rstrip("\n") + ";\n"

with open(path, "w") as f:
    f.writelines(lines)

print("Patched: added missing semicolon after evm_state_writes block")
