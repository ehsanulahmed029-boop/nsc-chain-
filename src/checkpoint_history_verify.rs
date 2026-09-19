use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HistoricalCheckpoint {

    pub height: u64,

    pub merkle_root: String,

    pub audit_root: String,
}

pub struct CheckpointHistoryVerify {

    pub checkpoints:
        HashMap<u64, HistoricalCheckpoint>,
}

impl CheckpointHistoryVerify {

    pub fn new() -> Self {

        Self {
            checkpoints:
                HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        checkpoint:
            HistoricalCheckpoint,
    ) {

        self.checkpoints.insert(
            checkpoint.height,
            checkpoint,
        );
    }

    pub fn verify(
        &self,
        height: u64,
        merkle_root: &str,
        audit_root: &str,
    ) -> bool {

        match self.checkpoints.get(
            &height
        ) {

            Some(cp) => {

                cp.merkle_root
                    == merkle_root

                    &&

                cp.audit_root
                    == audit_root
            }

            None => false,
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== HISTORICAL CHECKPOINTS ====="
        );

        for (height, cp)
            in &self.checkpoints
        {

            println!(
                "Height={} Merkle={} Audit={}",
                height,
                cp.merkle_root,
                cp.audit_root
            );
        }
    }
}
