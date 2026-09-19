// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-11): add() is never called
// from anywhere in the codebase, so `archive` is permanently empty.
// show() prints "Total Archived: 0" forever, which reads as a status
// confirmation but confirms nothing. Depends on the dead
// network_checkpoint::Checkpoint type. Not fund-critical — the real
// checkpoint/recovery system lives in chain.rs (Blockchain.checkpoints /
// cert_registry). See network_checkpoint.rs header for full cluster context.
use crate::network_checkpoint::Checkpoint;

pub struct CheckpointArchive {

    pub archive:
        Vec<Checkpoint>,
}

impl CheckpointArchive {

    pub fn new() -> Self {

        Self {
            archive: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        checkpoint: Checkpoint,
    ) {

        self.archive.push(
            checkpoint
        );
    }

    pub fn latest(
        &self,
    ) -> Option<&Checkpoint> {

        self.archive.last()
    }

    pub fn total(
        &self,
    ) -> usize {

        self.archive.len()
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== CHECKPOINT ARCHIVE ====="
        );

        for cp in &self.archive {

            println!(
                "height={} epoch={} hash={}",
                cp.height,
                cp.epoch,
                cp.block_hash
            );
        }

        println!(
            "Total Archived: {}",
            self.archive.len()
        );
    }
}
