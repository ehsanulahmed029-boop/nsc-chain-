#!/usr/bin/env python3
import shutil, sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.batch3_step3.bak'
    shutil.copy(path, backup)
    for old, new in replacements:
        count = content.count(old)
        if count != 1:
            print(f"ABORT: {path} — expected 1 match, found {count} for:\n{old[:150]}...")
            sys.exit(1)
        content = content.replace(old, new)
    with open(path, 'w') as f:
        f.write(content)
    print(f"OK: {path} patched (backup at {backup})")

DEAD_WARNING = """// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): Only ever called from the
// now-removed periodic-tick block that used the dead network-checkpoint
// cluster (network_checkpoint.rs and friends). Confirmed unused after that
// removal. Not fund-critical — the real checkpoint/recovery system lives in
// chain.rs (Blockchain.checkpoints / cert_registry). See network_checkpoint.rs
// header for the full cluster explanation.

"""

patch_file('src/checkpoint_replication.rs', [
    ('pub struct CheckpointReplication', DEAD_WARNING + 'pub struct CheckpointReplication')
])

patch_file('src/checkpoint_consensus.rs', [
    ('pub struct CheckpointConsensus;', DEAD_WARNING + 'pub struct CheckpointConsensus;')
])

print("\nBatch 3 step 3 patches applied.")
print("Now run:")
print("  cargo build --release 2>&1 | tail -60")
