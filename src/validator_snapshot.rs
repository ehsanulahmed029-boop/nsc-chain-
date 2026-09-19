
#[derive(Debug, Clone)]
pub struct ValidatorSnapshot {
    pub epoch: u64,
    pub validator: String,
    pub stake: u64,
    pub reputation: i64,
    pub active: bool,
}

#[derive(Debug)]
pub struct SnapshotManager {
    pub snapshots: Vec<ValidatorSnapshot>,
}

impl SnapshotManager {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    pub fn save_snapshot(
        &mut self,
        epoch: u64,
        validator: String,
        stake: u64,
        reputation: i64,
        active: bool,
    ) {
        self.snapshots.push(
            ValidatorSnapshot {
                epoch,
                validator,
                stake,
                reputation,
                active,
            }
        );
    }

    pub fn show_all(
        &self,
    ) {
        println!(
            "\n===== VALIDATOR SNAPSHOTS ====="
        );

        for s in &self.snapshots {
            println!(
                "epoch={} validator={} stake={} reputation={} active={}",
                s.epoch,
                s.validator,
                s.stake,
                s.reputation,
                s.active
            );
        }
    }

    pub fn epoch_snapshots(
        &self,
        epoch: u64,
    ) {
        println!(
            "\n===== SNAPSHOTS FOR EPOCH {} =====",
            epoch
        );

        for s in &self.snapshots {
            if s.epoch == epoch {
                println!(
                    "{} stake={} reputation={}",
                    s.validator,
                    s.stake,
                    s.reputation
                );
            }
        }
    }

    pub fn validator_history(
        &self,
        validator: &str,
    ) {
        println!(
            "\n===== HISTORY {} =====",
            validator
        );

        for s in &self.snapshots {
            if s.validator == validator {
                println!(
                    "epoch={} rep={} stake={}",
                    s.epoch,
                    s.reputation,
                    s.stake
                );
            }
        }
    }

    pub fn total_snapshots(
        &self,
    ) -> usize {
        self.snapshots.len()
    }
}
