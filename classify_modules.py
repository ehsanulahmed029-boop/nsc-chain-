import re

with open("/root/nsc-chain/src/main.rs") as f:
    content = f.read()

mod_names = re.findall(r"^mod (\w+);", content, re.MULTILINE)
print(f"[INFO] Total modules declared: {len(mod_names)}\n")

lines = content.split("\n")
non_decl_lines = [l for l in lines if not l.strip().startswith("mod ") and not l.strip().startswith("use ")]
non_decl_text = "\n".join(non_decl_lines)

results = []
for name in mod_names:
    usage_count = non_decl_text.count(f"{name}::")
    results.append((name, usage_count))

results.sort(key=lambda x: x[1])

zero_usage = [r for r in results if r[1] == 0]
low_usage = [r for r in results if 0 < r[1] <= 2]
active = [r for r in results if r[1] > 2]

print(f"[ZERO references outside mod/use] ({len(zero_usage)} modules):")
for name, c in zero_usage:
    print(f"  {name}")

print(f"\n[LOW references, 1-2] ({len(low_usage)} modules):")
for name, c in low_usage:
    print(f"  {name}: {c}")

print(f"\n[ACTIVE, 3+ references] ({len(active)} modules):")
for name, c in active:
    print(f"  {name}: {c}")
