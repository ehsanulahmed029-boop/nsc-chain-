use crate::checkpoint_archive::
    CheckpointArchive;

// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-11): operates only on
// CheckpointArchive, which is permanently empty (see checkpoint_archive.rs).
// prune() is therefore a no-op every cycle, and show() prints
// "Current Archive Size: 0" forever. Not fund-critical — the real
// checkpoint/recovery system lives in chain.rs (Blockchain.checkpoints /
// cert_registry).
pub struct CheckpointPruning;

impl CheckpointPruning {

    pub fn prune(
        archive:
            &mut CheckpointArchive,
        max_size: usize,
    ) {

        while archive.archive.len()
            > max_size
        {

            archive.archive.remove(0);
        }
    }

    pub fn show(
        archive:
            &CheckpointArchive,
    ) {

        println!(
            "\n===== CHECKPOINT PRUNING ====="
        );

        println!(
            "Current Archive Size: {}",
            archive.archive.len()
        );
    }
}
