import shutil, sys

CHAIN_RS = "/root/nsc-chain/src/chain.rs"
STORAGE_RS = "/root/nsc-chain/src/storage.rs"

def do(c, old, new, name):
    n = c.count(old)
    if n != 1:
        print(f"[ABORT] {name}: found {n} matches (expected 1)")
        sys.exit(1)
    print(f"[OK] {name}")
    return c.replace(old, new)

with open(CHAIN_RS) as f:
    c = f.read()
shutil.copy2(CHAIN_RS, CHAIN_RS + ".bak_before_cert_wireup_p4")

c = do(c,
    "            pending_spend_requests: self.pending_spend_requests.clone(),\n"
    "            checkpoints: self.checkpoints.clone(),\n"
    "        });\n"
    "    }",
    "            pending_spend_requests: self.pending_spend_requests.clone(),\n"
    "            checkpoints: self.checkpoints.clone(),\n"
    "            cert_registry: self.cert_registry.clone(),\n"
    "        });\n"
    "    }",
    "save() persistence")

c = do(c,
    "            self.pending_spend_requests = state.pending_spend_requests;\n"
    "            self.checkpoints = state.checkpoints;\n"
    "            println!(\"[CHAIN] Full state restored from disk.\");",
    "            self.pending_spend_requests = state.pending_spend_requests;\n"
    "            self.checkpoints = state.checkpoints;\n"
    "            self.cert_registry = state.cert_registry;\n"
    "            println!(\"[CHAIN] Full state restored from disk.\");",
    "apply_full_state() restore")

with open(CHAIN_RS, "w") as f:
    f.write(c)

with open(STORAGE_RS) as f:
    s = f.read()
shutil.copy2(STORAGE_RS, STORAGE_RS + ".bak_before_cert_wireup_p4")

s = do(s,
    "    #[serde(default)]\n"
    "    pub checkpoints: HashMap<u64, String>,\n"
    "}",
    "    #[serde(default)]\n"
    "    pub checkpoints: HashMap<u64, String>,\n"
    "    #[serde(default)]\n"
    "    pub cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry,\n"
    "}",
    "FullState field")

with open(STORAGE_RS, "w") as f:
    f.write(s)

print("[PART4 DONE] All patches complete.")
print("Next: cargo build --release 2>&1 | tail -60")
