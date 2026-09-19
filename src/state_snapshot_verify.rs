use sha2::{
    Digest,
    Sha256,
};

#[derive(Debug, Clone)]
pub struct StateSnapshot {

    pub height: u64,

    pub state_hash: String,
}

// ⚠️ DEAD-BY-DESIGN (active-4+ audit, 2026-08-11): the only callers,
// verify_state_snapshot() and create_state_snapshot() in main.rs, are
// never called from anywhere in the codebase. This is an abandoned
// duplicate of the real, verified full-state persistence path in
// chain.rs (storage::save_full_state()/load_full_state()). No fund
// risk — the real path is independent and already verified.
pub struct StateSnapshotVerify;

impl StateSnapshotVerify {

    pub fn hash(
        data: &str,
    ) -> String {

        let mut hasher =
            Sha256::new();

        hasher.update(
            data.as_bytes()
        );

        format!(
            "{:x}",
            hasher.finalize()
        )
    }

    pub fn verify(
        snapshot:
            &StateSnapshot,
        state_data: &str,
    ) -> bool {

        let calculated =
            Self::hash(
                state_data
            );

        calculated
            == snapshot.state_hash
    }

    pub fn show(
        snapshot:
            &StateSnapshot,
    ) {

        println!(
            "\n===== STATE SNAPSHOT ====="
        );

        println!(
            "Height: {}",
            snapshot.height
        );

        println!(
            "Hash: {}",
            snapshot.state_hash
        );
    }
}
