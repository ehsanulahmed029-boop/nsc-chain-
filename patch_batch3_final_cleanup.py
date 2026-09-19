#!/usr/bin/env python3
"""
Batch 3 final cleanup: remove dead network_checkpoint / merkle_registry
variable declarations from main.rs. The network_checkpoint cluster
(network_checkpoint.rs, checkpoint_scheduler.rs, checkpoint_merkle_registry.rs,
checkpoint_integrity.rs, checkpoint_consensus.rs, checkpoint_replication.rs)
is now fully confirmed dead-by-design with no remaining active call sites.
"""
import shutil
import sys
from pathlib import Path
from datetime import datetime

ROOT = Path("/root/nsc-chain")
BACKUP_DIR = Path("/root/nsc-chain-archive/batch3_final_cleanup_" +
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

# 1. remove dead network_checkpoint declaration
old = "    let mut network_checkpoint = NetworkCheckpoint::new();\n"
new = (
    "    // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): network_checkpoint declaration\n"
    "    // removed — dead-by-design, see network_checkpoint.rs header. No remaining\n"
    "    // active call sites (create_checkpoint() that used it is itself unused).\n"
)
patch_file(p, old, new, "main.rs network_checkpoint declaration")

# 2. remove dead merkle_registry declaration
old = "    let merkle_registry = CheckpointMerkleRegistry::new();\n"
new = (
    "    // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): merkle_registry declaration\n"
    "    // removed — dead-by-design, see checkpoint_merkle_registry.rs header.\n"
    "    // register()/get()/verify()/show() had no remaining call sites.\n"
)
patch_file(p, old, new, "main.rs merkle_registry declaration")

print("\nAll patches applied successfully. Next: cargo build --release")
