#!/usr/bin/env python3
import shutil
import sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.bak'
    shutil.copy(path, backup)
    for old, new in replacements:
        count = content.count(old)
        if count != 1:
            print(f"ABORT: {path} — expected 1 match, found {count} for:\n{old[:100]}...")
            sys.exit(1)
        content = content.replace(old, new)
    with open(path, 'w') as f:
        f.write(content)
    print(f"OK: {path} patched (backup at {backup})")

DEAD_WARNING = """// ⚠️ DEAD-BY-DESIGN (Batch 2 audit, 2026-08-10): This module is never
// instantiated/called anywhere in main.rs or elsewhere. Its output would be
// misleading if displayed, since it reflects no real state. Do not wire this
// up without re-auditing the logic for correctness first. See audit notes.

"""

patch_file('src/checkpoint_proposal_execution.rs', [
    ('pub struct CheckpointProposalExecution;',
     DEAD_WARNING + 'pub struct CheckpointProposalExecution;')
])

patch_file('src/checkpoint_proposal_validation.rs', [
    ('pub struct CheckpointProposalValidation;',
     DEAD_WARNING + 'pub struct CheckpointProposalValidation;')
])

patch_file('src/checkpoint_weighted_quorum.rs', [
    ('pub struct CheckpointWeightedQuorum;',
     DEAD_WARNING + 'pub struct CheckpointWeightedQuorum;')
])

patch_file('src/treasury_audit_hashchain.rs', [
    ('pub struct AuditHashRecord {',
     DEAD_WARNING + 'pub struct AuditHashRecord {')
])

patch_file('src/checkpoint_quorum.rs', [
    ('pub struct CheckpointQuorum;',
     DEAD_WARNING + 'pub struct CheckpointQuorum;')
])

patch_file('src/checkpoint_slashing.rs', [
    ('pub struct CheckpointSlashing {',
     DEAD_WARNING + 'pub struct CheckpointSlashing {')
])

TREASURY_FREEZE_WARNING = """// ⚠️ DEAD-BY-DESIGN + MISLEADING NAME (Batch 2 audit, 2026-08-10):
// This struct is NOT the real treasury freeze mechanism. The REAL freeze
// flag is `Treasury.frozen` in treasury.rs, which IS correctly checked in
// execute_spend(). This TreasuryFreeze struct is a disconnected duplicate:
// its can_spend() is never called anywhere, so it enforces nothing. Do not
// use this struct to build freeze functionality — use Treasury.freeze()
// in treasury.rs instead. See audit notes.

"""
patch_file('src/treasury_freeze.rs', [
    ('pub struct TreasuryFreeze {',
     TREASURY_FREEZE_WARNING + 'pub struct TreasuryFreeze {')
])

patch_file('src/main.rs', [
    ('pub fn check_checkpoint_quorum(',
     '// ⚠️ DEAD-BY-DESIGN (Batch 2 audit, 2026-08-10): never called anywhere.\npub fn check_checkpoint_quorum(')
])
patch_file('src/main.rs', [
    ('pub fn check_network_checkpoint_quorum(',
     '// ⚠️ DEAD-BY-DESIGN (Batch 2 audit, 2026-08-10): never called anywhere.\npub fn check_network_checkpoint_quorum(')
])

# Occurrence 1: Step 6e block (startup), followed by treasury_hash_chain.show()
patch_file('src/main.rs', [
    ('''    // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() removed — see
    // treasury_protection.rs header comment. expected_balance is a
    // static constant (1_000_000) that never updates with real
    // deposits/spends, so this fired a false "TAMPERING DETECTED"
    // alarm on every legitimate balance change.
    treasury_audit_log.show();
    treasury_hash_chain.show();''',
     '''    // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() removed — see
    // treasury_protection.rs header comment. expected_balance is a
    // static constant (1_000_000) that never updates with real
    // deposits/spends, so this fired a false "TAMPERING DETECTED"
    // alarm on every legitimate balance change.
    // ⚠️ REMOVED (Batch 2 audit, 2026-08-10): treasury_audit_log.show() was
    // printing misleading output — .record() is never called anywhere, so
    // this log is permanently empty and does not reflect real treasury activity.
    treasury_hash_chain.show();''')
])

# Occurrence 2: 10m periodic loop block, followed by chain.treasury.show()
patch_file('src/main.rs', [
    ('''            // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() removed —
            // static expected_balance never updates with real activity.
            treasury_audit_log.show();
            chain.treasury.show();''',
     '''            // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() removed —
            // static expected_balance never updates with real activity.
            // ⚠️ REMOVED (Batch 2 audit, 2026-08-10): treasury_audit_log.show() was
            // printing misleading output — .record() is never called anywhere.
            chain.treasury.show();''')
])

print("\nAll patches applied successfully.")
print("Now run:")
print("  cargo build --release 2>&1 | tail -50")
print("  diff src/main.rs.bak src/main.rs")
