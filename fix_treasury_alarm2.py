import shutil, sys

MAIN_RS = "/root/nsc-chain/src/main.rs"

with open(MAIN_RS) as f:
    c = f.read()
shutil.copy2(MAIN_RS, MAIN_RS + ".bak_before_treasury_alarm_fix")

def do(c, old, new, name):
    n = c.count(old)
    if n != 1:
        print(f"[ABORT] {name}: found {n} matches (expected 1)")
        sys.exit(1)
    print(f"[OK] {name}")
    return c.replace(old, new)

# ── Site 1: startup, step 6d ──
c = do(c,
    "    {\n"
    "        let chain = blockchain.lock().expect(\"chain lock\");\n"
    "        treasury_guard.detect_tampering(chain.treasury.balance());\n"
    "    }",
    "    // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() removed — see\n"
    "    // treasury_protection.rs header comment. expected_balance is a\n"
    "    // static constant (1_000_000) that never updates with real\n"
    "    // deposits/spends, so this fired a false \"TAMPERING DETECTED\"\n"
    "    // alarm on every legitimate balance change.",
    "site 1: startup detect_tampering")

# ── Site 2: main loop, step 10m (per-tick) ──
c = do(c,
    "        // ── 10m: Treasury audit ───────────────────────────────\n"
    "        {\n"
    "            let chain = blockchain.lock().expect(\"chain lock\");\n"
    "            treasury_guard.detect_tampering(chain.treasury.balance());\n"
    "            treasury_audit_log.show();\n"
    "            chain.treasury.show();\n"
    "            chain.treasury.show_history();\n"
    "        }",
    "        // ── 10m: Treasury audit ───────────────────────────────\n"
    "        {\n"
    "            let chain = blockchain.lock().expect(\"chain lock\");\n"
    "            // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() removed —\n"
    "            // static expected_balance never updates with real activity.\n"
    "            treasury_audit_log.show();\n"
    "            chain.treasury.show();\n"
    "            chain.treasury.show_history();\n"
    "        }",
    "site 2: per-tick detect_tampering")

# ── Site 3: treasury_deposit() helper (called on every real deposit) ──
c = do(c,
    "    guard.detect_tampering(treasury.balance());\n"
    "    let _ = treasury.deposit(amount);\n"
    "    guard.detect_tampering(treasury.balance());\n"
    "    println!(\n"
    "        \"[TREASURY] Deposited {}. Balance: {}.\",\n"
    "        amount,\n"
    "        treasury.balance()\n"
    "    );",
    "    // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() calls removed —\n"
    "    // static expected_balance never updates with real deposits, so\n"
    "    // this fired a false alarm on every legitimate deposit.\n"
    "    let _ = treasury.deposit(amount);\n"
    "    println!(\n"
    "        \"[TREASURY] Deposited {}. Balance: {}.\",\n"
    "        amount,\n"
    "        treasury.balance()\n"
    "    );",
    "site 3: treasury_deposit() helper")

with open(MAIN_RS, "w") as f:
    f.write(c)

print("[DONE] All 3 detect_tampering() call sites removed.")
