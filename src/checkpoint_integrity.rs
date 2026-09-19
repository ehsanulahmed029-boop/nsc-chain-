use crate::network_checkpoint::{
    NetworkCheckpoint,
};

// ⚠️ DEAD-BY-DESIGN + TAUTOLOGICAL BUG (Batch 3 audit, 2026-08-10):
// Every call site in main.rs passes `checkpoint.latest().block_hash` as
// BOTH the checkpoint being checked AND the expected_hash to compare
// against — i.e. verify() always compares a value against itself. This
// makes it mathematically incapable of ever returning false, regardless
// of whether real tampering occurred. It never provided real integrity
// checking. Part of the dead network-checkpoint cluster — see
// network_checkpoint.rs header. Do not re-enable without fixing the
// self-comparison bug AND wiring it to an independently-sourced expected
// hash.

pub struct CheckpointIntegrity;

impl CheckpointIntegrity {

    pub fn verify(
        checkpoint: &NetworkCheckpoint,
        expected_hash: &str,
    ) -> bool {

        match checkpoint.latest() {

            Some(cp) => {

                cp.block_hash
                    ==
                    expected_hash
            }

            None => false,
        }
    }

    pub fn show(
        checkpoint: &NetworkCheckpoint,
        expected_hash: &str,
    ) {

        let valid =
            Self::verify(
                checkpoint,
                expected_hash,
            );

        println!(
            "\n===== CHECKPOINT INTEGRITY ====="
        );

        println!(
            "Expected Hash: {}",
            expected_hash
        );

        println!(
            "Valid: {}",
            valid
        );
    }
}
