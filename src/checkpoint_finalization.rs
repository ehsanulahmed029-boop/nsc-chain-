use std::collections::HashSet;

// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-11): finalize() is never
// called from anywhere in the codebase, so `finalized` is permanently
// empty. show() prints the "FINALIZED CHECKPOINTS" header with an
// always-empty list, which reads as a status confirmation but confirms
// nothing. Not fund-critical — the real checkpoint/recovery system lives
// in chain.rs (Blockchain.checkpoints / cert_registry).
pub struct CheckpointFinalization {

    pub finalized:
        HashSet<u64>,
}

impl CheckpointFinalization {

    pub fn new() -> Self {

        Self {

            finalized:
                HashSet::new(),
        }
    }

    pub fn finalize(
        &mut self,
        height: u64,
    ) {

        self.finalized.insert(
            height
        );

        println!(
            "Checkpoint finalized at {}",
            height
        );
    }

    pub fn is_finalized(
        &self,
        height: u64,
    ) -> bool {

        self.finalized.contains(
            &height
        )
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== FINALIZED CHECKPOINTS ====="
        );

        for height
            in &self.finalized
        {

            println!(
                "Finalized Height: {}",
                height
            );
        }
    }
}
