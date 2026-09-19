use std::collections::HashMap;

#[derive(Debug)]
// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): Part of a fully separate,
// non-persistent "network checkpoint" cluster (network_checkpoint.rs,
// checkpoint_scheduler.rs, checkpoint_merkle_registry.rs,
// checkpoint_integrity.rs) that is disconnected from the real, fund-critical
// checkpoint/recovery system in chain.rs (Blockchain.checkpoints /
// cert_registry / recover_from_checkpoint()). This cluster is in-memory
// only and is wiped on every restart. Confirmed NOT a fund-risk — the real
// recovery path does not depend on it — but its status output was
// misleading and has been removed. See audit notes.

pub struct CheckpointMerkleRegistry {

    pub roots:
        HashMap<u64, String>,
}

impl CheckpointMerkleRegistry {

    pub fn new() -> Self {

        Self {
            roots:
                HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        height: u64,
        merkle_root: String,
    ) {

        self.roots.insert(
            height,
            merkle_root,
        );
    }

    pub fn get(
        &self,
        height: u64,
    ) -> Option<&String> {

        self.roots.get(
            &height
        )
    }

    pub fn verify(
        &self,
        height: u64,
        root: &str,
    ) -> bool {

        match self.roots.get(
            &height
        ) {

            Some(stored) =>
                stored == root,

            None => false,
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== CHECKPOINT MERKLE REGISTRY ====="
        );

        for (height, root)
            in &self.roots
        {

            println!(
                "Height={} Root={}",
                height,
                root
            );
        }

        println!(
            "Total Checkpoints: {}",
            self.roots.len()
        );
    }
}
