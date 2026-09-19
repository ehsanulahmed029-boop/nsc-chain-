#!/usr/bin/env python3
"""
Reputation wiring fix: TrustScore::show() and ElectionV3::elect() were
being passed trust_rep, a separate ReputationManager instance that is
never populated via .register() — so score_of() always returned the
default fallback of 100, meaning trust score and leader election never
reflected real validator behavior (successful_blocks/missed_blocks/
reward/punish history tracked in the real `reputation` instance).
Fix: wire both call sites to the real, populated `reputation` instance.
Also removes _admission_rep, a separate unused ReputationManager instance.
"""
import shutil
import sys
from pathlib import Path
from datetime import datetime

ROOT = Path("/root/nsc-chain")
BACKUP_DIR = Path("/root/nsc-chain-archive/reputation_wiring_fix_" +
                   datetime.now().strftime("%Y%m%d_%H%M%S"))
BACKUP_DIR.mkdir(parents=True, exist_ok=True)

def backup(path: Path):
    dest = BACKUP_DIR / path.name
    shutil.copy2(path, dest)
    print(f"  backed up -> {dest}")

def patch_file(path: Path, old: str, new: str, label: str):
    text = path.read_text()
    count = text.count(old)
    if count != 1:
        print(f"ABORT [{label}]: expected 1 match in {path}, found {count}")
        sys.exit(1)
    backup(path)
    text = text.replace(old, new, 1)
    path.write_text(text)
    print(f"  patched [{label}] in {path}")

p = ROOT / "src/main.rs"

# 1. Remove trust_rep declaration (was a separate, never-populated instance)
old = "    let trust_rep = ReputationManager::new();\n"
new = (
    "    // ⚠️ FIXED (reputation-wiring audit, 2026-08-11): trust_rep removed —\n"
    "    // it was a separate ReputationManager instance never populated via\n"
    "    // .register(), so TrustScore/ElectionV3 always saw the fallback score\n"
    "    // of 100 for every validator instead of real reputation. Call sites\n"
    "    // below now use the real, populated `reputation` instance.\n"
)
patch_file(p, old, new, "main.rs remove trust_rep declaration")

# 2. Remove _admission_rep declaration (separate unused instance)
old = "    let _admission_rep = ReputationManager::new();\n"
new = (
    "    // ⚠️ REMOVED (reputation-wiring audit, 2026-08-11): _admission_rep\n"
    "    // declaration removed — unused separate ReputationManager instance.\n"
)
patch_file(p, old, new, "main.rs remove _admission_rep declaration")

# 3. ElectionV3::elect() call site — replace &trust_rep with &reputation
old = (
    "        let _winners = ElectionV3::elect(\n"
    "            &validator_registry,\n"
    "            &trust_rep,\n"
    "            &trust_hb,\n"
    "            &trust_history,\n"
    "            current_epoch,\n"
    "            2,\n"
    "        );\n"
)
new = (
    "        let _winners = ElectionV3::elect(\n"
    "            &validator_registry,\n"
    "            &reputation,\n"
    "            &trust_hb,\n"
    "            &trust_history,\n"
    "            current_epoch,\n"
    "            2,\n"
    "        );\n"
)
patch_file(p, old, new, "main.rs ElectionV3::elect() reputation wiring")

# 4. TrustScore::show() loop — replace &trust_rep with &reputation
old = (
    "        for addr in &validator_addrs {\n"
    "            TrustScore::show(\n"
    "                addr,\n"
    "                &trust_rep,\n"
    "                &trust_hb,\n"
    "                &trust_history,\n"
    "                current_epoch,\n"
    "            );\n"
    "        }\n"
)
new = (
    "        for addr in &validator_addrs {\n"
    "            TrustScore::show(\n"
    "                addr,\n"
    "                &reputation,\n"
    "                &trust_hb,\n"
    "                &trust_history,\n"
    "                current_epoch,\n"
    "            );\n"
    "        }\n"
)
patch_file(p, old, new, "main.rs TrustScore::show() reputation wiring")

print("\nAll patches applied successfully. Next: cargo build --release")
