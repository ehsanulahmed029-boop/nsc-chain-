use std::collections::HashMap;

use crate::network_checkpoint::Checkpoint;

// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): Only ever called from the
// now-removed periodic-tick block that used the dead network-checkpoint
// cluster (network_checkpoint.rs and friends). Confirmed unused after that
// removal. Not fund-critical — the real checkpoint/recovery system lives in
// chain.rs (Blockchain.checkpoints / cert_registry). See network_checkpoint.rs
// header for the full cluster explanation.

pub struct CheckpointReplication {

    pub replicas:
        HashMap<String, Vec<Checkpoint>>,
}

impl CheckpointReplication {

    pub fn new() -> Self {

        Self {
            replicas:
                HashMap::new(),
        }
    }

    pub fn replicate(
        &mut self,
        validator: String,
        checkpoint: Checkpoint,
    ) {

        self.replicas
            .entry(validator)
            .or_insert(Vec::new())
            .push(checkpoint);
    }

    pub fn replica_count(
        &self,
        validator: &str,
    ) -> usize {

        self.replicas
            .get(validator)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== CHECKPOINT REPLICATION ====="
        );

        for (validator, cps)
            in &self.replicas
        {

            println!(
                "{} => {} checkpoints",
                validator,
                cps.len()
            );
        }
    }
}
