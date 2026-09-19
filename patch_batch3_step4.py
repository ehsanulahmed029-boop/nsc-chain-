#!/usr/bin/env python3
import shutil, sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.batch3_step4.bak'
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

OLD = '''        // ── 10d: Checkpoint scheduling ────────────────────────
        {
            let chain = blockchain.lock().expect("chain lock");
            let height = chain.chain_height() as u64;
            let epoch = epoch_manager.current_epoch;

            if height > 0 {
                if let Some(last) = chain.blocks.last() {
                    CheckpointScheduler::process(
                        height,
                        epoch,
                        last.hash.clone(),
                        CHECKPOINT_INTERVAL as u64,
                        &mut network_checkpoint,
                    );
                }
            }
        }'''

NEW = '''        // ⚠️ REMOVED (Batch 3 audit, 2026-08-10): CheckpointScheduler::process()
        // only ever populated the dead, in-memory-only network_checkpoint
        // cluster and printed the misleading "Automatic Checkpoint Created"
        // log line — it never touched the real, fund-critical checkpoint
        // system in chain.rs (Blockchain.checkpoints / cert_registry, created
        // automatically every 5 blocks in mine_pending_transactions()). See
        // network_checkpoint.rs header for the full cluster explanation.'''

patch_file('src/main.rs', [(OLD, NEW)])

print("\nBatch 3 step 4 patch applied.")
print("Now run:")
print("  cargo build --release 2>&1 | tail -60")
