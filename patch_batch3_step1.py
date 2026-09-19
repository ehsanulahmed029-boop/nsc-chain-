#!/usr/bin/env python3
import shutil, sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.batch3.bak'
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

DEAD_WARNING = """// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): Part of a fully separate,
// non-persistent "network checkpoint" cluster (network_checkpoint.rs,
// checkpoint_scheduler.rs, checkpoint_merkle_registry.rs,
// checkpoint_integrity.rs) that is disconnected from the real, fund-critical
// checkpoint/recovery system in chain.rs (Blockchain.checkpoints /
// cert_registry / recover_from_checkpoint()). This cluster is in-memory
// only and is wiped on every restart. Confirmed NOT a fund-risk — the real
// recovery path does not depend on it — but its status output was
// misleading and has been removed. See audit notes.

"""

# 1. network_checkpoint.rs
patch_file('src/network_checkpoint.rs', [
    ('pub struct Checkpoint {', DEAD_WARNING + 'pub struct Checkpoint {')
])

# 2. checkpoint_scheduler.rs
patch_file('src/checkpoint_scheduler.rs', [
    ('pub struct CheckpointScheduler;', DEAD_WARNING + 'pub struct CheckpointScheduler;')
])

# 3. checkpoint_merkle_registry.rs
patch_file('src/checkpoint_merkle_registry.rs', [
    ('pub struct CheckpointMerkleRegistry {', DEAD_WARNING + 'pub struct CheckpointMerkleRegistry {')
])

# 4. checkpoint_integrity.rs — extra tautology warning
INTEGRITY_WARNING = """// ⚠️ DEAD-BY-DESIGN + TAUTOLOGICAL BUG (Batch 3 audit, 2026-08-10):
// Every call site in main.rs passes `checkpoint.latest().block_hash` as
// BOTH the checkpoint being checked AND the expected_hash to compare
// against — i.e. verify() always compares a value against itself. This
// makes it mathematically incapable of ever returning false, regardless
// of whether real tampering occurred. It never provided real integrity
// checking. Part of the dead network-checkpoint cluster — see
// network_checkpoint.rs header. Do not re-enable without fixing the
// self-comparison bug AND wiring it to an independently-sourced expected
// hash.

"""
patch_file('src/checkpoint_integrity.rs', [
    ('pub struct CheckpointIntegrity;', INTEGRITY_WARNING + 'pub struct CheckpointIntegrity;')
])

# 5. main.rs — mark create_checkpoint() wrapper dead
patch_file('src/main.rs', [
    ('''/// Creates a new checkpoint at the current chain height.
/// Called every CHECKPOINT_INTERVAL blocks.
pub fn create_checkpoint(''',
     '''// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): never called anywhere.
// Part of the dead network-checkpoint cluster — see network_checkpoint.rs.
/// Creates a new checkpoint at the current chain height.
/// Called every CHECKPOINT_INTERVAL blocks.
pub fn create_checkpoint(''')
])

# 6. main.rs — mark verify_checkpoint_archive() wrapper dead
patch_file('src/main.rs', [
    ('''/// Verifies the integrity of the checkpoint archive.
pub fn verify_checkpoint_archive(''',
     '''// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): never called anywhere.
/// Verifies the integrity of the checkpoint archive.
pub fn verify_checkpoint_archive(''')
])

# 7. main.rs — remove startup network_checkpoint.show() block (~line 668)
patch_file('src/main.rs', [
    ('''    // ── Step 6h: Checkpoint integrity ────────────────────────
    println!("[CHECKPOINT] Checking checkpoint integrity...");
    network_checkpoint.show();
    if let Some(cp) = network_checkpoint.latest() {
        println!(
            "[CHECKPOINT] Latest checkpoint height={} epoch={}",
            cp.height,''',
     '''    // ⚠️ REMOVED (Batch 3 audit, 2026-08-10): network_checkpoint.show() and
    // the block below printed status from the dead network-checkpoint
    // cluster (in-memory only, disconnected from real checkpoint/recovery
    // in chain.rs). See network_checkpoint.rs header for details.
    /* ORIGINAL:
    println!("[CHECKPOINT] Checking checkpoint integrity...");
    network_checkpoint.show();
    if let Some(cp) = network_checkpoint.latest() {
        println!(
            "[CHECKPOINT] Latest checkpoint height={} epoch={}",
            cp.height,''')
])

print("\nBatch 3 step 1 patches applied (partial — main.rs periodic-tick block still needs manual review due to /* comment nesting risk).")
print("Now run:")
print("  cargo build --release 2>&1 | tail -100")
