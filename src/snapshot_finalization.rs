#[derive(Debug, Clone)]
pub struct FinalizedSnapshot {

    pub height: u64,

    pub snapshot_hash: String,

    pub finalized: bool,
}

pub struct SnapshotFinalization;

impl SnapshotFinalization {

    pub fn finalize(
        height: u64,
        snapshot_hash: String,
    ) -> FinalizedSnapshot {

        FinalizedSnapshot {

            height,

            snapshot_hash,

            finalized: true,
        }
    }

    pub fn verify(
        snapshot:
            &FinalizedSnapshot,
    ) -> bool {

        snapshot.finalized
    }

    pub fn show(
        snapshot:
            &FinalizedSnapshot,
    ) {

        println!(
            "\n===== SNAPSHOT FINALIZATION ====="
        );

        println!(
            "Height: {}",
            snapshot.height
        );

        println!(
            "Hash: {}",
            snapshot.snapshot_hash
        );

        println!(
            "Finalized: {}",
            snapshot.finalized
        );
    }
}
