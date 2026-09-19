#!/usr/bin/env python3
"""
Batch 3 cleanup: mark checkpoint_archive / checkpoint_finalization / checkpoint_pruning
as dead-by-design, remove misleading empty-state call sites from main.rs.
"""
import shutil
import sys
from pathlib import Path
from datetime import datetime

ROOT = Path("/root/nsc-chain")
BACKUP_DIR = Path("/root/nsc-chain-archive/batch3_checkpoint_archive_cleanup_" +
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

# 1. checkpoint_archive.rs
p = ROOT / "src/checkpoint_archive.rs"
old = "use crate::network_checkpoint::Checkpoint;"
new = (
    "// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-11): add() is never called\n"
    "// from anywhere in the codebase, so `archive` is permanently empty.\n"
    "// show() prints \"Total Archived: 0\" forever, which reads as a status\n"
    "// confirmation but confirms nothing. Depends on the dead\n"
    "// network_checkpoint::Checkpoint type. Not fund-critical — the real\n"
    "// checkpoint/recovery system lives in chain.rs (Blockchain.checkpoints /\n"
    "// cert_registry). See network_checkpoint.rs header for full cluster context.\n"
    "use crate::network_checkpoint::Checkpoint;"
)
patch_file(p, old, new, "checkpoint_archive.rs header")

# 2. checkpoint_finalization.rs
p = ROOT / "src/checkpoint_finalization.rs"
old = "pub struct CheckpointFinalization {"
new = (
    "// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-11): finalize() is never\n"
    "// called from anywhere in the codebase, so `finalized` is permanently\n"
    "// empty. show() prints the \"FINALIZED CHECKPOINTS\" header with an\n"
    "// always-empty list, which reads as a status confirmation but confirms\n"
    "// nothing. Not fund-critical — the real checkpoint/recovery system lives\n"
    "// in chain.rs (Blockchain.checkpoints / cert_registry).\n"
    "pub struct CheckpointFinalization {"
)
patch_file(p, old, new, "checkpoint_finalization.rs header")

# 3. checkpoint_pruning.rs
p = ROOT / "src/checkpoint_pruning.rs"
old = "pub struct CheckpointPruning;"
new = (
    "// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-11): operates only on\n"
    "// CheckpointArchive, which is permanently empty (see checkpoint_archive.rs).\n"
    "// prune() is therefore a no-op every cycle, and show() prints\n"
    "// \"Current Archive Size: 0\" forever. Not fund-critical — the real\n"
    "// checkpoint/recovery system lives in chain.rs (Blockchain.checkpoints /\n"
    "// cert_registry).\n"
    "pub struct CheckpointPruning;"
)
patch_file(p, old, new, "checkpoint_pruning.rs header")

# 4. main.rs: remove dead variable declarations
p = ROOT / "src/main.rs"
old = (
    "    let mut checkpoint_archive = CheckpointArchive::new();\n"
    "    let checkpoint_finalization = CheckpointFinalization::new();\n"
)
new = (
    "    // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): checkpoint_archive /\n"
    "    // checkpoint_finalization declarations removed — both were dead-by-design,\n"
    "    // see checkpoint_archive.rs / checkpoint_finalization.rs headers.\n"
)
patch_file(p, old, new, "main.rs declarations")

# 5. main.rs: remove archive/pruning call sites
p = ROOT / "src/main.rs"
old = (
    "        checkpoint_archive.show();\n"
    "        CheckpointPruning::prune(&mut checkpoint_archive, 10);\n"
    "        CheckpointPruning::show(&checkpoint_archive);\n"
)
new = (
    "        // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): checkpoint_archive.show(),\n"
    "        // CheckpointPruning::prune()/show() removed — archive.add() is never\n"
    "        // called anywhere, so the archive was permanently empty; prune() was\n"
    "        // a no-op and both show() calls printed misleading \"0\" status lines\n"
    "        // every cycle. See checkpoint_archive.rs / checkpoint_pruning.rs headers.\n"
)
patch_file(p, old, new, "main.rs archive/pruning call sites")

# 6. main.rs: remove finalization call site
p = ROOT / "src/main.rs"
old = "        checkpoint_finalization.show();\n"
new = (
    "        // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): checkpoint_finalization.show()\n"
    "        // removed — finalize() is never called anywhere, so this always printed\n"
    "        // an empty \"FINALIZED CHECKPOINTS\" list. See checkpoint_finalization.rs header.\n"
)
patch_file(p, old, new, "main.rs finalization call site")

print("\nAll patches applied successfully. Next: cargo build --release")
