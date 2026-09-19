#!/usr/bin/env python3
import shutil
import sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.step1.bak'
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

DEAD_WARNING = """// ⚠️ DEAD-BY-DESIGN (Batch 2.5 audit, 2026-08-10): This module is never
// wired to any real enforcement path. Its output would be misleading if
// displayed. Part of a 4-struct emergency-freeze cluster (EmergencyFreeze,
// EmergencyRecovery, ChainFreeze, TreasuryFreeze) that was abandoned
// mid-implementation — none of them connect to each other or to real state.
// Real freeze mechanism being built: Blockchain.chain_frozen (chain.rs) for
// chain, Treasury.frozen (treasury.rs) for treasury. See audit notes.

"""

# 1. emergency_freeze.rs — mark dead
patch_file('src/emergency_freeze.rs', [
    ('pub struct EmergencyFreeze {',
     DEAD_WARNING + 'pub struct EmergencyFreeze {')
])

# 2. emergency_recovery.rs — mark dead
patch_file('src/emergency_recovery.rs', [
    ('pub struct EmergencyRecovery {',
     DEAD_WARNING + 'pub struct EmergencyRecovery {')
])

# 3. chain_freeze.rs — mark dead
patch_file('src/chain_freeze.rs', [
    ('pub struct ChainFreeze {',
     DEAD_WARNING + 'pub struct ChainFreeze {')
])

# 4. main.rs — remove misleading show() calls (lines ~655-656)
patch_file('src/main.rs', [
    ('    emergency_recovery_mode.show();\n    emergency_freeze.show();\n',
     '    // ⚠️ REMOVED (Batch 2.5 audit, 2026-08-10): emergency_recovery_mode\n'
     '    // and emergency_freeze were printing misleading status — neither is\n'
     '    // wired to any real freeze/recovery path. See audit notes.\n')
])

# 5. main.rs — remove chain freeze guard block (lines ~748-754)
patch_file('src/main.rs', [
    ('''    // ── Step 6aa: Chain freeze guard ─────────────────────────
    println!(
        "[FREEZE] Chain frozen: {}",
        chain_freeze.is_frozen()
    );''',
     '''    // ⚠️ REMOVED (Batch 2.5 audit, 2026-08-10): this printed chain_freeze
    // .is_frozen(), but ChainFreeze was never wired to any real enforcement
    // path — freezing it changed nothing. Real chain_frozen field is now on
    // Blockchain itself; see mine_pending_transactions() for enforcement.''')
])

# 6. main.rs — remove periodic emergency freeze check block (lines ~1254-1259)
patch_file('src/main.rs', [
    ('''        // ── 10ag: Emergency freeze check ─────────────────────
        emergency_freeze.show();
        println!(
            "[FREEZE] Chain frozen: {}",
            chain_freeze.is_frozen()
        );''',
     '''        // ⚠️ REMOVED (Batch 2.5 audit, 2026-08-10): emergency_freeze.show()
        // and chain_freeze.is_frozen() were misleading — see audit notes above.''')
])

# 7. chain.rs — add real chain_frozen field to Blockchain struct
patch_file('src/chain.rs', [
    ('''    pub cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry,
}''',
     '''    pub cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry,
    /// Real chain freeze flag (Batch 2.5, 2026-08-10). Unlike the old
    /// ChainFreeze struct (dead-by-design), this is enforced directly in
    /// mine_pending_transactions().
    pub chain_frozen: bool,
}''')
])

# 8. chain.rs — initialize chain_frozen in Blockchain::empty()
patch_file('src/chain.rs', [
    ('''            pending_spend_requests: HashMap::new(),
            cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry::new(),
        }
    }''',
     '''            pending_spend_requests: HashMap::new(),
            cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry::new(),
            chain_frozen: false,
        }
    }''')
])

# 9. chain.rs — wire into save()
patch_file('src/chain.rs', [
    ('''            checkpoints: self.checkpoints.clone(),
            cert_registry: self.cert_registry.clone(),
        });''',
     '''            checkpoints: self.checkpoints.clone(),
            cert_registry: self.cert_registry.clone(),
            chain_frozen: self.chain_frozen,
        });''')
])

# 10. chain.rs — wire into apply_full_state()
patch_file('src/chain.rs', [
    ('''            self.cert_registry = state.cert_registry;
            println!("[CHAIN] Full state restored from disk.");''',
     '''            self.cert_registry = state.cert_registry;
            self.chain_frozen = state.chain_frozen;
            println!("[CHAIN] Full state restored from disk.");''')
])

# 11. storage.rs — add chain_frozen to FullState struct
patch_file('src/storage.rs', [
    ('''    #[serde(default)]
    pub cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry,
}''',
     '''    #[serde(default)]
    pub cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry,
    #[serde(default)]
    pub chain_frozen: bool,
}''')
])

print("\nStep 1 (partial) patches applied.")
print("NOTE: mine_pending_transactions() enforcement NOT yet added — needs body review first.")
print("Now run:")
print("  cargo build --release 2>&1 | tail -80")
