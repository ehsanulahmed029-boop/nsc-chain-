#[derive(Debug, Clone)]
// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): Part of a fully separate,
// non-persistent "network checkpoint" cluster (network_checkpoint.rs,
// checkpoint_scheduler.rs, checkpoint_merkle_registry.rs,
// checkpoint_integrity.rs) that is disconnected from the real, fund-critical
// checkpoint/recovery system in chain.rs (Blockchain.checkpoints /
// cert_registry / recover_from_checkpoint()). This cluster is in-memory
// only and is wiped on every restart. Confirmed NOT a fund-risk — the real
// recovery path does not depend on it — but its status output was
// misleading and has been removed. See audit notes.

pub struct Checkpoint {

    pub height: u64,

    pub block_hash: String,

    pub epoch: u64,
}

pub struct NetworkCheckpoint {

    pub checkpoints:
        Vec<Checkpoint>,
}

impl NetworkCheckpoint {

    pub fn new() -> Self {

        Self {
            checkpoints:
                Vec::new(),
        }
    }

    pub fn create(
        &mut self,
        height: u64,
        block_hash: String,
        epoch: u64,
    ) {

        self.checkpoints.push(
            Checkpoint {

                height,

                block_hash,

                epoch,
            }
        );
    }

    pub fn latest(
        &self,
    ) -> Option<&Checkpoint> {

        self.checkpoints.last()
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== NETWORK CHECKPOINTS ====="
        );

        for cp in &self.checkpoints {

            println!(
                "height={} epoch={} hash={}",
                cp.height,
                cp.epoch,
                cp.block_hash
            );
        }
    }
}
