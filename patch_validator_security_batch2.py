#!/usr/bin/env python3
"""
Validator-security batch 2: mark strike_system as dead-by-design.
TrustScore confirmed real/active (validator_addrs sourced from
validator_registry, TrustScore::show() called from main loop) — no
change needed there.
"""
import shutil
import sys
from pathlib import Path
from datetime import datetime

ROOT = Path("/root/nsc-chain")
BACKUP_DIR = Path("/root/nsc-chain-archive/validator_security_batch2_" +
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

# 1. strike_system.rs header
p = ROOT / "src/strike_system.rs"
old = "use std::collections::HashMap;\n\n#[derive(Debug)]\npub struct StrikeSystem {"
new = (
    "use std::collections::HashMap;\n\n"
    "// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): the only\n"
    "// populate path, add_validator_strike() in main.rs, is never called from\n"
    "// anywhere in the codebase. The top-level instance in main.rs was already\n"
    "// underscore-prefixed (_strikes) by the original author, signalling it was\n"
    "// known unused. strikes/banned are therefore permanently empty. No strike\n"
    "// enforcement is currently active via this module. (Note: StrikeEngineV2 /\n"
    "// add_weighted_strike() is a separate module — not covered by this audit.)\n"
    "#[derive(Debug)]\npub struct StrikeSystem {"
)
patch_file(p, old, new, "strike_system.rs header")

# 2. main.rs: remove dead _strikes declaration
p = ROOT / "src/main.rs"
old = "    let _strikes = StrikeSystem::new();\n"
new = (
    "    // ⚠️ REMOVED (validator-security audit, 2026-08-11): _strikes\n"
    "    // declaration removed — dead-by-design, see strike_system.rs header.\n"
)
patch_file(p, old, new, "main.rs _strikes declaration")

# 3. main.rs: mark add_validator_strike() wrapper as dead
p = ROOT / "src/main.rs"
old = (
    "/// Adds a strike to a validator.\n"
    "/// Bans the validator if the strike limit is reached.\n"
    "pub fn add_validator_strike("
)
new = (
    "// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): never called\n"
    "// anywhere in the codebase. See strike_system.rs header.\n"
    "/// Adds a strike to a validator.\n"
    "/// Bans the validator if the strike limit is reached.\n"
    "pub fn add_validator_strike("
)
patch_file(p, old, new, "main.rs add_validator_strike() marker")

print("\nAll patches applied successfully. Next: cargo build --release")
