#!/usr/bin/env python3
import shutil, sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.batch3_step2.bak'
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

# ── Startup block (~line 666-674): remove network_checkpoint.show() + latest() block ──
OLD_STARTUP = '''    // ── Step 6h: Checkpoint integrity ────────────────────────
    println!("[CHECKPOINT] Checking checkpoint integrity...");
    network_checkpoint.show();
    if let Some(cp) = network_checkpoint.latest() {
        println!(
            "[CHECKPOINT] Latest checkpoint height={} epoch={}",
            cp.height,
            cp.epoch
        );
    }'''

NEW_STARTUP = '''    // ⚠️ REMOVED (Batch 3 audit, 2026-08-10): this printed status from the
    // dead network-checkpoint cluster (network_checkpoint.rs and friends) —
    // in-memory only, disconnected from the real checkpoint/recovery system
    // in chain.rs. See network_checkpoint.rs header for details.'''

patch_file('src/main.rs', [(OLD_STARTUP, NEW_STARTUP)])

# ── Periodic-tick block (~line 1053-1070): remove CheckpointIntegrity::show +
#    checkpoint_archive.add + replication + CheckpointConsensus::show ──
OLD_TICK = '''        // ── 10e: Checkpoint integrity ─────────────────────────
        if let Some(cp) = network_checkpoint.latest() {
            CheckpointIntegrity::show(
                &network_checkpoint,
                &cp.block_hash,
            );

            // Archive latest checkpoint
            checkpoint_archive.add(cp.clone());

            // Replicate to known validators
            for (addr, _) in &validator_registry.validators {
                checkpoint_replication.replicate(
                    addr.clone(),
                    cp.clone(),
                );
            }

            CheckpointConsensus::show(&checkpoint_replication);
        }'''

NEW_TICK = '''        // ⚠️ REMOVED (Batch 3 audit, 2026-08-10): CheckpointIntegrity::show() was
        // tautological (always compared a checkpoint's hash against itself,
        // so it could never detect real tampering — see checkpoint_integrity.rs
        // header). checkpoint_archive/checkpoint_replication/CheckpointConsensus
        // calls here operated on the same dead network-checkpoint cluster and
        // provided no real fund-critical guarantee; removed alongside it.'''

patch_file('src/main.rs', [(OLD_TICK, NEW_TICK)])

# ── Also remove the merkle_registry.show() call (~line 1278, per earlier grep) ──
OLD_MERKLE = '''        merkle_registry.show();'''
NEW_MERKLE = '''        // ⚠️ REMOVED (Batch 3 audit, 2026-08-10): merkle_registry.show() always
        // printed "Total Checkpoints: 0" — register() is never called anywhere,
        // so this registry is permanently empty. Part of the dead
        // network-checkpoint cluster; see network_checkpoint.rs header.'''

patch_file('src/main.rs', [(OLD_MERKLE, NEW_MERKLE)])

print("\nBatch 3 step 2 patches applied.")
print("Now run:")
print("  cargo build --release 2>&1 | tail -100")
