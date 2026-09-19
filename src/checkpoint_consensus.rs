use std::collections::HashMap;

use crate::checkpoint_replication::
    CheckpointReplication;

// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): Only ever called from the
// now-removed periodic-tick block that used the dead network-checkpoint
// cluster (network_checkpoint.rs and friends). Confirmed unused after that
// removal. Not fund-critical — the real checkpoint/recovery system lives in
// chain.rs (Blockchain.checkpoints / cert_registry). See network_checkpoint.rs
// header for the full cluster explanation.

pub struct CheckpointConsensus;

impl CheckpointConsensus {

    pub fn verify(
        replication:
            &CheckpointReplication,
    ) -> bool {

        let mut votes:
            HashMap<String, usize>
            = HashMap::new();

        for checkpoints
            in replication.replicas.values()
        {

            if let Some(cp)
                = checkpoints.last()
            {

                *votes
                    .entry(
                        cp.block_hash.clone()
                    )
                    .or_insert(0)
                    += 1;
            }
        }

        let total =
            replication.replicas.len();

        for count
            in votes.values()
        {

            if count * 2
                > total
            {

                return true;
            }
        }

        false
    }

    pub fn show(
        replication:
            &CheckpointReplication,
    ) {

        println!(
            "\n===== CHECKPOINT CONSENSUS ====="
        );

        println!(
            "Consensus Valid: {}",
            Self::verify(
                replication
            )
        );
    }
}
