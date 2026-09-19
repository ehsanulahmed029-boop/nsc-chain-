#!/usr/bin/env python3
"""
Wire Staking into Blockchain: struct field, constructor init,
save()/apply_full_state() persistence, FullState struct field.
"""
import shutil
import sys
from pathlib import Path
from datetime import datetime

ROOT = Path("/root/nsc-chain")
BACKUP_DIR = Path("/root/nsc-chain-archive/staking_wiring_" +
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

# 1. chain.rs: add staking field to Blockchain struct
p = ROOT / "src/chain.rs"
old = "    pub pending_freeze_requests: HashMap<String, crate::emergency_freeze_multisig::EmergencyFreezeRequest>,\n}"
new = (
    "    pub pending_freeze_requests: HashMap<String, crate::emergency_freeze_multisig::EmergencyFreezeRequest>,\n"
    "    /// Real validator staking (2026-08-11 wiring pass). Balance-backed\n"
    "    /// via crate::amm::BalanceLedger (implemented below). See\n"
    "    /// staking.rs for stake()/unstake()/claim_unbonded()/slash_stake().\n"
    "    pub staking: crate::staking::Staking,\n}"
)
patch_file(p, old, new, "chain.rs Blockchain struct field")

# 2. chain.rs: init in empty()
p = ROOT / "src/chain.rs"
old = "            pending_freeze_requests: HashMap::new(),\n        }\n    }\n\n    // ── Genesis validation"
new = (
    "            pending_freeze_requests: HashMap::new(),\n"
    "            staking: crate::staking::Staking::new(),\n"
    "        }\n    }\n\n    // ── Genesis validation"
)
patch_file(p, old, new, "chain.rs empty() init")

# 3. chain.rs: persist in save()
p = ROOT / "src/chain.rs"
old = "            pending_freeze_requests: self.pending_freeze_requests.clone(),\n        });\n    }"
new = (
    "            pending_freeze_requests: self.pending_freeze_requests.clone(),\n"
    "            staking: self.staking.clone(),\n"
    "        });\n    }"
)
patch_file(p, old, new, "chain.rs save() persist")

# 4. chain.rs: restore in apply_full_state()
p = ROOT / "src/chain.rs"
old = "            self.pending_freeze_requests = state.pending_freeze_requests;\n            println!(\"[CHAIN] Full state restored from disk.\");"
new = (
    "            self.pending_freeze_requests = state.pending_freeze_requests;\n"
    "            self.staking = state.staking;\n"
    "            println!(\"[CHAIN] Full state restored from disk.\");"
)
patch_file(p, old, new, "chain.rs apply_full_state() restore")

# 5. storage.rs: FullState struct field
p = ROOT / "src/storage.rs"
old = "    #[serde(default)]\n    pub pending_freeze_requests: HashMap<String, crate::emergency_freeze_multisig::EmergencyFreezeRequest>,\n}"
new = (
    "    #[serde(default)]\n"
    "    pub pending_freeze_requests: HashMap<String, crate::emergency_freeze_multisig::EmergencyFreezeRequest>,\n"
    "    #[serde(default)]\n"
    "    pub staking: crate::staking::Staking,\n}"
)
patch_file(p, old, new, "storage.rs FullState field")

print("\nAll patches applied successfully. Next: check Staking has Default (needed for #[serde(default)]), then cargo build --release")
