import shutil, sys

CHAIN_RS = "/root/nsc-chain/src/chain.rs"

with open(CHAIN_RS) as f:
    c = f.read()
shutil.copy2(CHAIN_RS, CHAIN_RS + ".bak_before_cert_wireup_p2")

def do(c, old, new, name):
    n = c.count(old)
    if n != 1:
        print(f"[ABORT] {name}: found {n} matches (expected 1)")
        sys.exit(1)
    print(f"[OK] {name}")
    return c.replace(old, new)

c = do(c,
    "    pub pending_spend_requests: HashMap<String, crate::multisig::TreasurySpendRequest>,\n}",
    "    pub pending_spend_requests: HashMap<String, crate::multisig::TreasurySpendRequest>,\n"
    "    pub cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry,\n"
    "}",
    "struct field")

c = do(c,
    "            pending_spend_requests: HashMap::new(),\n        }\n    }",
    "            pending_spend_requests: HashMap::new(),\n"
    "            cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry::new(),\n"
    "        }\n    }",
    "init in empty()")

c = do(c,
    "    pub fn create_checkpoint(&mut self) {\n"
    "        if let Some(last) = self.blocks.last() {\n"
    "            self.checkpoints.insert(last.index, last.hash.clone());\n"
    "            println!(\n"
    "                \"[CHECKPOINT] Created at block {}.\",\n"
    "                last.index\n"
    "            );\n"
    "        }\n"
    "    }",
    "    pub fn create_checkpoint(&mut self) {\n"
    "        if let Some(last) = self.blocks.last() {\n"
    "            self.checkpoints.insert(last.index, last.hash.clone());\n"
    "            println!(\n"
    "                \"[CHECKPOINT] Created at block {}.\",\n"
    "                last.index\n"
    "            );\n"
    "            let cert = crate::recovery_snapshot_certificate::RecoverySnapshotCertificate::generate(\n"
    "                last.index,\n"
    "                last.hash.clone(),\n"
    "            );\n"
    "            self.cert_registry.register(cert);\n"
    "            println!(\n"
    "                \"[CERT] Recovery certificate generated for checkpoint at block {}.\",\n"
    "                last.index\n"
    "            );\n"
    "        }\n"
    "    }",
    "create_checkpoint hook")

with open(CHAIN_RS, "w") as f:
    f.write(c)

print("[PART2 DONE]")
