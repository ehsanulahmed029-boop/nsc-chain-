#!/usr/bin/env python3
import shutil
import sys
from pathlib import Path
from datetime import datetime

ROOT = Path("/root/nsc-chain")
BACKUP_DIR = Path("/root/nsc-chain-archive/active_batch1_fix_" +
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

old = (
    "/// Verifies that the state data matches the snapshot hash.\n"
    "pub fn verify_state_snapshot("
)
new = (
    "// ⚠️ DEAD-BY-DESIGN (active-4+ audit, 2026-08-11): never called anywhere.\n"
    "// See state_snapshot_verify.rs header.\n"
    "/// Verifies that the state data matches the snapshot hash.\n"
    "pub fn verify_state_snapshot("
)
patch_file(p, old, new, "main.rs verify_state_snapshot() marker")

old = (
    "/// Creates a new state snapshot from current state data.\n"
    "pub fn create_state_snapshot("
)
new = (
    "// ⚠️ DEAD-BY-DESIGN (active-4+ audit, 2026-08-11): never called anywhere.\n"
    "// See state_snapshot_verify.rs header.\n"
    "/// Creates a new state snapshot from current state data.\n"
    "pub fn create_state_snapshot("
)
patch_file(p, old, new, "main.rs create_state_snapshot() marker")

print("\nAll patches applied successfully. Next: cargo build --release")
