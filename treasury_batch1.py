import shutil, sys

MAIN_RS = "/root/nsc-chain/src/main.rs"
LOCK_RS = "/root/nsc-chain/src/treasury_recovery_lock.rs"
FREEZE_RS = "/root/nsc-chain/src/treasury_emergency_freeze.rs"
MERKLE_RS = "/root/nsc-chain/src/treasury_merkle_audit.rs"

def do(c, old, new, name):
    n = c.count(old)
    if n != 1:
        print(f"[ABORT] {name}: found {n} matches (expected 1)")
        sys.exit(1)
    print(f"[OK] {name}")
    return c.replace(old, new)

with open(MAIN_RS) as f:
    m = f.read()
shutil.copy2(MAIN_RS, MAIN_RS + ".bak_before_treasury_batch1")

m = do(m,
    "        // ── 10aq: Treasury emergency freeze ──────────────────\n"
    "        treasury_emergency.show();",
    "        // ── 10aq: Treasury emergency freeze ───────────────────\n"
    "        // [DEAD-CODE-FIX 2026-08-09] treasury_emergency.show() removed.\n"
    "        // TreasuryEmergencyFreeze is a fully disconnected decoration —\n"
    "        // its freeze()/unfreeze() are never called anywhere, so it always\n"
    "        // prints \"Frozen: false\" regardless of real treasury state. The\n"
    "        // REAL freeze mechanism (Treasury.frozen / Treasury::freeze()) is\n"
    "        // correctly checked inside execute_spend(), but ALSO has no call\n"
    "        // site anywhere yet — there is currently no way to actually\n"
    "        // trigger a treasury freeze. Building that (e.g. a multisig-gated\n"
    "        // API route calling chain.treasury.freeze()) is a separate,\n"
    "        // not-yet-built feature, tracked as an open item.",
    "remove treasury_emergency.show()")

with open(MAIN_RS, "w") as f:
    f.write(m)

with open(LOCK_RS) as f:
    l = f.read()
if "UNUSED BY DESIGN" not in l:
    shutil.copy2(LOCK_RS, LOCK_RS + ".bak_before_treasury_batch1")
    notice = (
        "// WARNING: UNUSED (found 2026-08-09). Instantiated in main.rs as\n"
        "// `_recovery_lock` (underscore prefix = intentionally unused).\n"
        "// can_recover()/execute_recovery() are never called anywhere.\n"
        "// Logic itself looks correct but is fully disconnected from any\n"
        "// real recovery path. Do not assume this provides any active\n"
        "// protection.\n"
        "pub struct TreasuryRecoveryLock {"
    )
    l2 = l.replace("pub struct TreasuryRecoveryLock {", notice)
    if l2 == l:
        print("[ABORT] treasury_recovery_lock dead-mark: pattern not found")
        sys.exit(1)
    with open(LOCK_RS, "w") as f:
        f.write(l2)
    print("[OK] treasury_recovery_lock.rs marked dead")
else:
    print("[SKIP] treasury_recovery_lock.rs already marked")

with open(FREEZE_RS) as f:
    e = f.read()
if "UNUSED BY DESIGN" not in e:
    shutil.copy2(FREEZE_RS, FREEZE_RS + ".bak_before_treasury_batch1")
    notice = (
        "// WARNING: DISCONNECTED FROM REAL STATE (found 2026-08-09).\n"
        "// This struct's `frozen` field is entirely separate from the\n"
        "// REAL treasury freeze flag (Treasury.frozen in treasury.rs),\n"
        "// which IS correctly checked by execute_spend(). freeze()/\n"
        "// unfreeze() on THIS struct are never called anywhere, so its\n"
        "// .show() output (\"Frozen: false\") is always wrong-by-omission —\n"
        "// it reflects nothing about real treasury state. Do not assume\n"
        "// this provides any active protection or accurate status.\n"
        "pub struct TreasuryEmergencyFreeze {"
    )
    e2 = e.replace("pub struct TreasuryEmergencyFreeze {", notice)
    if e2 == e:
        print("[ABORT] treasury_emergency_freeze dead-mark: pattern not found")
        sys.exit(1)
    with open(FREEZE_RS, "w") as f:
        f.write(e2)
    print("[OK] treasury_emergency_freeze.rs marked dead")
else:
    print("[SKIP] treasury_emergency_freeze.rs already marked")

with open(MERKLE_RS) as f:
    k = f.read()
if "UNUSED BY DESIGN" not in k:
    shutil.copy2(MERKLE_RS, MERKLE_RS + ".bak_before_treasury_batch1")
    notice = (
        "// WARNING: UNUSED (found 2026-08-09). merkle_root() logic is\n"
        "// correct (real SHA256 merkle tree), but the only caller,\n"
        "// compute_treasury_merkle_root() in main.rs, is itself never\n"
        "// called anywhere. Do not assume any merkle audit of treasury\n"
        "// records is actually happening.\n"
        "pub struct TreasuryMerkleAudit;"
    )
    k2 = k.replace("pub struct TreasuryMerkleAudit;", notice)
    if k2 == k:
        print("[ABORT] treasury_merkle_audit dead-mark: pattern not found")
        sys.exit(1)
    with open(MERKLE_RS, "w") as f:
        f.write(k2)
    print("[OK] treasury_merkle_audit.rs marked dead")
else:
    print("[SKIP] treasury_merkle_audit.rs already marked")

print("[DONE] Batch 1 complete.")
