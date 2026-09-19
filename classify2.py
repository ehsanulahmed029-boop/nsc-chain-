import re

with open("/root/nsc-chain/src/main.rs") as f:
    content = f.read()

mod_names = re.findall(r"^mod (\w+);", content, re.MULTILINE)

lines = content.split("\n")
use_lines = [l for l in lines if l.strip().startswith("use ")]
body_lines = [l for l in lines if not l.strip().startswith("use ") and not l.strip().startswith("mod ")]
body_text = "\n".join(body_lines)

mod_to_types = {}
for m in mod_names:
    types = set()
    for ul in use_lines:
        pat1 = re.match(rf"use\s+{re.escape(m)}::(\w+);", ul.strip())
        pat2 = re.match(rf"use\s+{re.escape(m)}::\{{([^}}]+)\}};", ul.strip())
        if pat1:
            types.add(pat1.group(1))
        elif pat2:
            for t in pat2.group(1).split(","):
                types.add(t.strip())
    mod_to_types[m] = types

results = []
for m in mod_names:
    types = mod_to_types.get(m, set())
    total_usage = 0
    for t in types:
        total_usage += len(re.findall(rf"\b{re.escape(t)}\b", body_text))
    results.append((m, len(types), total_usage))

results.sort(key=lambda x: x[2])

no_import = [r for r in results if r[1] == 0]
zero_usage = [r for r in results if r[1] > 0 and r[2] == 0]
low_usage = [r for r in results if r[1] > 0 and 0 < r[2] <= 3]
active = [r for r in results if r[2] > 3]

print(f"[INFO] Total modules: {len(mod_names)}\n")

print(f"[NO IMPORTED TYPES FOUND — needs manual check] ({len(no_import)}):")
for m, ti, u in no_import:
    print(f"  {m}")

print(f"\n[IMPORTED BUT ZERO USAGE — likely dead] ({len(zero_usage)}):")
for m, ti, u in zero_usage:
    print(f"  {m}")

print(f"\n[LOW USAGE 1-3 — check for fake-resilience pattern] ({len(low_usage)}):")
for m, ti, u in low_usage:
    print(f"  {m}: {u}")

print(f"\n[ACTIVE 4+ usages] ({len(active)}):")
for m, ti, u in active:
    print(f"  {m}: {u}")
