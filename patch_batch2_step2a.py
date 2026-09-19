#!/usr/bin/env python3
import shutil, sys

def patch_file(path, replacements):
    with open(path, 'r') as f:
        content = f.read()
    backup = path + '.step2a.bak'
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

# 1. chain.rs — add fields to Blockchain struct
patch_file('src/chain.rs', [
    ('''    /// Real chain freeze flag (Batch 2.5, 2026-08-10). Unlike the old
    /// ChainFreeze struct (dead-by-design), this is enforced directly in
    /// mine_pending_transactions().
    pub chain_frozen: bool,
}''',
     '''    /// Real chain freeze flag (Batch 2.5, 2026-08-10). Unlike the old
    /// ChainFreeze struct (dead-by-design), this is enforced directly in
    /// mine_pending_transactions().
    pub chain_frozen: bool,
    pub emergency_freeze_multisig: crate::emergency_freeze_multisig::EmergencyFreezeMultiSig,
    pub pending_freeze_requests: HashMap<String, crate::emergency_freeze_multisig::EmergencyFreezeRequest>,
}''')
])

# 2. chain.rs — initialize in Blockchain::empty()
patch_file('src/chain.rs', [
    ('''            pending_spend_requests: HashMap::new(),
            cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry::new(),
            chain_frozen: false,
        }
    }''',
     '''            pending_spend_requests: HashMap::new(),
            cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry::new(),
            chain_frozen: false,
            emergency_freeze_multisig: crate::emergency_freeze_multisig::EmergencyFreezeMultiSig::new(
                vec![
                    "NSCe258f89e5fa12872".to_string(),
                    "NSC8bfe30da185e00f3".to_string(),
                    "NSCa06f136253f19163".to_string(),
                ],
                3,
            ),
            pending_freeze_requests: HashMap::new(),
        }
    }''')
])

# 3. chain.rs — wire into save()
patch_file('src/chain.rs', [
    ('''            cert_registry: self.cert_registry.clone(),
            chain_frozen: self.chain_frozen,
        });''',
     '''            cert_registry: self.cert_registry.clone(),
            chain_frozen: self.chain_frozen,
            emergency_freeze_multisig: self.emergency_freeze_multisig.clone(),
            pending_freeze_requests: self.pending_freeze_requests.clone(),
        });''')
])

# 4. chain.rs — wire into apply_full_state()
patch_file('src/chain.rs', [
    ('''            self.chain_frozen = state.chain_frozen;
            println!("[CHAIN] Full state restored from disk.");''',
     '''            self.chain_frozen = state.chain_frozen;
            self.emergency_freeze_multisig = state.emergency_freeze_multisig;
            self.pending_freeze_requests = state.pending_freeze_requests;
            println!("[CHAIN] Full state restored from disk.");''')
])

# 5. storage.rs — add fields to FullState struct
patch_file('src/storage.rs', [
    ('''    #[serde(default)]
    pub chain_frozen: bool,
}''',
     '''    #[serde(default)]
    pub chain_frozen: bool,
    #[serde(default)]
    pub emergency_freeze_multisig: crate::emergency_freeze_multisig::EmergencyFreezeMultiSig,
    #[serde(default)]
    pub pending_freeze_requests: HashMap<String, crate::emergency_freeze_multisig::EmergencyFreezeRequest>,
}''')
])

# 6. main.rs — register mod (alongside other mod declarations, near multisig)
patch_file('src/main.rs', [
    ('mod checkpoint_slashing;',
     'mod checkpoint_slashing;\nmod emergency_freeze_multisig;')
])

# 7. main.rs — register use
patch_file('src/main.rs', [
    ('use checkpoint_slashing::CheckpointSlashing;',
     'use checkpoint_slashing::CheckpointSlashing;\nuse emergency_freeze_multisig::{EmergencyFreezeMultiSig, EmergencyFreezeRequest};')
])

print("\nStep 2a patches applied.")
print("NOTE: EmergencyFreezeMultiSig/Request needs Default derive or explicit impl for #[serde(default)] to work.")
print("Now run:")
print("  cargo build --release 2>&1 | tail -80")
