#!/usr/bin/env python3
import shutil
import sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.step1b.bak'
    shutil.copy(path, backup)
    for old, new in replacements:
        count = content.count(old)
        if count != 1:
            print(f"ABORT: {path} — expected 1 match, found {count} for:\n{old[:120]}...")
            sys.exit(1)
        content = content.replace(old, new)
    with open(path, 'w') as f:
        f.write(content)
    print(f"OK: {path} patched (backup at {backup})")

patch_file('src/chain.rs', [
    ('''    pub fn mine_pending_transactions(
        &mut self,
        miner_address: String,
    ) {
        // Clean expired mempool entries first.
        self.cleanup_mempool();''',
     '''    pub fn mine_pending_transactions(
        &mut self,
        miner_address: String,
    ) {
        // ── Chain freeze enforcement (Batch 2.5, 2026-08-10) ──────
        // Real enforcement, unlike the old dead ChainFreeze struct.
        // No block is mined while the chain is frozen.
        if self.chain_frozen {
            println!("[MINE] Chain is frozen. Mining suspended.");
            return;
        }

        // Clean expired mempool entries first.
        self.cleanup_mempool();''')
])

print("\nStep 1b patch applied.")
print("Now run:")
print("  cargo build --release 2>&1 | tail -60")
