import shutil, sys

CHAIN_RS = "/root/nsc-chain/src/chain.rs"

with open(CHAIN_RS) as f:
    c = f.read()
shutil.copy2(CHAIN_RS, CHAIN_RS + ".bak_before_cert_wireup_p3")

old = (
    "    pub fn recover_from_checkpoint(&mut self) {\n"
    "        match self.find_last_valid_checkpoint() {\n"
    "            Some(height) => {\n"
    "                self.blocks.truncate((height + 1) as usize);\n"
    "                self.rebuild_balances();\n"
    "                // [FIX-RECOVERY-WIPE] rebuild_balances() only clears state;\n"
    "                // it does not replay balances from blocks. Without this call,\n"
    "                // checkpoint recovery would zero every wallet balance and then\n"
    "                // chain.save() would persist that wipe permanently to disk.\n"
    "                self.apply_full_state();\n"
    "                self.rebuild_processed_txs();\n"
    "                println!(\"[CHAIN] Recovered to checkpoint at block {}.\", height);\n"
    "            }\n"
    "            None => {\n"
    "                eprintln!(\"[CHAIN] No valid checkpoint to recover to.\");\n"
    "            }\n"
    "        }\n"
    "    }"
)

new = (
    "    pub fn recover_from_checkpoint(&mut self) {\n"
    "        match self.find_last_valid_checkpoint() {\n"
    "            Some(height) => {\n"
    "                let checkpoint_hash = match self.checkpoints.get(&height) {\n"
    "                    Some(h) => h.clone(),\n"
    "                    None => {\n"
    "                        eprintln!(\"[CERT] No checkpoint hash at height {} — aborting.\", height);\n"
    "                        return;\n"
    "                    }\n"
    "                };\n"
    "                let seed = format!(\"{}:{}\", height, checkpoint_hash);\n"
    "                let expected_cert_id = {\n"
    "                    use sha2::{Digest, Sha256};\n"
    "                    let mut hasher = Sha256::new();\n"
    "                    hasher.update(seed.as_bytes());\n"
    "                    format!(\"{:x}\", hasher.finalize())\n"
    "                };\n"
    "                let cert = match self.cert_registry.get(&expected_cert_id) {\n"
    "                    Some(c) => c.clone(),\n"
    "                    None => {\n"
    "                        eprintln!(\"[CERT] No recovery certificate for height {} — aborting. Manual intervention required.\", height);\n"
    "                        return;\n"
    "                    }\n"
    "                };\n"
    "                if !crate::recovery_snapshot_certificate::RecoverySnapshotCertificate::verify(&cert) {\n"
    "                    eprintln!(\"[CERT] Certificate for height {} FAILED verification — aborting. Manual intervention required.\", height);\n"
    "                    return;\n"
    "                }\n"
    "                println!(\"[CERT] Recovery certificate for height {} verified OK.\", height);\n"
    "                self.blocks.truncate((height + 1) as usize);\n"
    "                self.rebuild_balances();\n"
    "                // [FIX-RECOVERY-WIPE] rebuild_balances() only clears state;\n"
    "                // it does not replay balances from blocks. Without this call,\n"
    "                // checkpoint recovery would zero every wallet balance and then\n"
    "                // chain.save() would persist that wipe permanently to disk.\n"
    "                self.apply_full_state();\n"
    "                self.rebuild_processed_txs();\n"
    "                println!(\"[CHAIN] Recovered to checkpoint at block {}.\", height);\n"
    "            }\n"
    "            None => {\n"
    "                eprintln!(\"[CHAIN] No valid checkpoint to recover to.\");\n"
    "            }\n"
    "        }\n"
    "    }"
)

n = c.count(old)
if n != 1:
    print(f"[ABORT] recover_from_checkpoint hook: found {n} matches (expected 1)")
    sys.exit(1)

with open(CHAIN_RS, "w") as f:
    f.write(c.replace(old, new))

print("[OK] recover_from_checkpoint hook")
print("[PART3 DONE]")
