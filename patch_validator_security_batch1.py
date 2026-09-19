#!/usr/bin/env python3
"""
Validator-security batch 1: mark key_rotation / sybil_detector / node_fingerprint
as dead-by-design. Their populate/wrapper functions (rotate_validator_key(),
run_sybil_detection(), generate_node_fingerprint()) are never called anywhere,
so these modules provide no active security enforcement despite convincing
status logs and function names.
"""
import shutil
import sys
from pathlib import Path
from datetime import datetime

ROOT = Path("/root/nsc-chain")
BACKUP_DIR = Path("/root/nsc-chain-archive/validator_security_batch1_" +
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

# 1. key_rotation.rs header
p = ROOT / "src/key_rotation.rs"
old = "use std::collections::HashMap;\n\n#[derive(Debug)]\npub struct KeyRotationManager {"
new = (
    "use std::collections::HashMap;\n\n"
    "// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): the only path\n"
    "// that populates this manager is rotate_validator_key() in main.rs, which\n"
    "// is never called from anywhere in the codebase. active_keys/key_history\n"
    "// are therefore permanently empty. show() (still called from the main\n"
    "// loop) prints the \"KEY ROTATION\" header with nothing under it, which\n"
    "// reads as a status confirmation but confirms nothing. No real key\n"
    "// rotation enforcement currently exists.\n"
    "#[derive(Debug)]\npub struct KeyRotationManager {"
)
patch_file(p, old, new, "key_rotation.rs header")

# 2. sybil_detector.rs header
p = ROOT / "src/sybil_detector.rs"
old = "pub struct SybilDetector;"
new = (
    "// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): the only\n"
    "// caller, run_sybil_detection() in main.rs, is never called from\n"
    "// anywhere in the codebase. Additionally, ValidatorIdentity (the input\n"
    "// type) is never instantiated anywhere, so even if wired in there is no\n"
    "// real data source for it yet. No sybil detection is currently active.\n"
    "pub struct SybilDetector;"
)
patch_file(p, old, new, "sybil_detector.rs header")

# 3. node_fingerprint.rs header
p = ROOT / "src/node_fingerprint.rs"
old = "pub struct NodeFingerprint;"
new = (
    "// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): the only\n"
    "// caller, generate_node_fingerprint() in main.rs, is never called from\n"
    "// anywhere in the codebase. No node fingerprinting is currently active.\n"
    "pub struct NodeFingerprint;"
)
patch_file(p, old, new, "node_fingerprint.rs header")

# 4. main.rs: remove key_rotation declaration
p = ROOT / "src/main.rs"
old = "    let key_rotation = KeyRotationManager::new();\n"
new = (
    "    // ⚠️ REMOVED (validator-security audit, 2026-08-11): key_rotation\n"
    "    // declaration removed — dead-by-design, see key_rotation.rs header.\n"
)
patch_file(p, old, new, "main.rs key_rotation declaration")

# 5. main.rs: remove key_rotation.show() call
p = ROOT / "src/main.rs"
old = "        key_rotation.show();\n"
new = (
    "        // ⚠️ REMOVED (validator-security audit, 2026-08-11): key_rotation.show()\n"
    "        // removed — rotate_validator_key() is never called anywhere, so this\n"
    "        // always printed an empty \"KEY ROTATION\" list. See key_rotation.rs header.\n"
)
patch_file(p, old, new, "main.rs key_rotation.show() call")

# 6. main.rs: mark rotate_validator_key() wrapper as dead
p = ROOT / "src/main.rs"
old = (
    "/// Rotates a validator's signing key.\n"
    "/// Called when a validator rotates their key pair.\n"
    "pub fn rotate_validator_key("
)
new = (
    "// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): never called\n"
    "// anywhere in the codebase. See key_rotation.rs header.\n"
    "/// Rotates a validator's signing key.\n"
    "/// Called when a validator rotates their key pair.\n"
    "pub fn rotate_validator_key("
)
patch_file(p, old, new, "main.rs rotate_validator_key() marker")

# 7. main.rs: mark run_sybil_detection() wrapper as dead
p = ROOT / "src/main.rs"
old = (
    "/// Runs sybil detection on the current validator set.\n"
    "/// Called periodically from the node loop.\n"
    "pub fn run_sybil_detection("
)
new = (
    "// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): never called\n"
    "// anywhere in the codebase. ValidatorIdentity is also never instantiated\n"
    "// anywhere, so there is no data source even if this were wired in.\n"
    "// See sybil_detector.rs header.\n"
    "/// Runs sybil detection on the current validator set.\n"
    "/// Called periodically from the node loop.\n"
    "pub fn run_sybil_detection("
)
patch_file(p, old, new, "main.rs run_sybil_detection() marker")

# 8. main.rs: mark generate_node_fingerprint() wrapper as dead
p = ROOT / "src/main.rs"
old = (
    "/// Generates a node fingerprint for sybil resistance.\n"
    "pub fn generate_node_fingerprint("
)
new = (
    "// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): never called\n"
    "// anywhere in the codebase. See node_fingerprint.rs header.\n"
    "/// Generates a node fingerprint for sybil resistance.\n"
    "pub fn generate_node_fingerprint("
)
patch_file(p, old, new, "main.rs generate_node_fingerprint() marker")

print("\nAll patches applied successfully. Next: cargo build --release")
