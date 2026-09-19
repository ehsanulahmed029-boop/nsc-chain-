use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct SignedSnapshot {

    pub height: u64,

    pub snapshot_hash: String,

    pub signatures:
        HashSet<String>,
}

pub struct SnapshotSignature;

impl SnapshotSignature {

    pub fn new(
        height: u64,
        snapshot_hash: String,
    ) -> SignedSnapshot {

        SignedSnapshot {

            height,

            snapshot_hash,

            signatures:
                HashSet::new(),
        }
    }

    pub fn sign(
        snapshot:
            &mut SignedSnapshot,
        validator: String,
    ) {

        snapshot.signatures.insert(
            validator
        );
    }

    pub fn verify(
        snapshot:
            &SignedSnapshot,
        required: usize,
    ) -> bool {

        snapshot.signatures.len()
            >= required
    }

    pub fn show(
        snapshot:
            &SignedSnapshot,
    ) {

        println!(
            "\n===== SNAPSHOT SIGNATURES ====="
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
            "Signatures: {}",
            snapshot.signatures.len()
        );
    }
}
