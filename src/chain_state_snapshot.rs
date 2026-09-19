#[derive(Debug, Clone)]
pub struct ChainState {

    pub treasury_balance: u128,

    pub block_height: u64,

    pub current_epoch: u64,

    pub governance_version: u32,

    pub validator_count: usize,
}

pub struct ChainStateSnapshot {

    pub snapshot:
        Option<ChainState>,
}

impl ChainStateSnapshot {

    pub fn new() -> Self {

        Self {
            snapshot: None,
        }
    }

    pub fn save(
        &mut self,
        treasury_balance: u128,
        block_height: u64,
        current_epoch: u64,
        governance_version: u32,
        validator_count: usize,
    ) {

        self.snapshot = Some(
            ChainState {

                treasury_balance,

                block_height,

                current_epoch,

                governance_version,

                validator_count,
            }
        );
    }

    pub fn load(
        &self,
    ) -> Option<&ChainState> {

        self.snapshot.as_ref()
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== CHAIN STATE SNAPSHOT ====="
        );

        match &self.snapshot {

            Some(state) => {

                println!(
                    "Block Height: {}",
                    state.block_height
                );

                println!(
                    "Epoch: {}",
                    state.current_epoch
                );

                println!(
                    "Governance Version: {}",
                    state.governance_version
                );

                println!(
                    "Validators: {}",
                    state.validator_count
                );
            }

            None => {

                println!(
                    "No Snapshot Found"
                );
            }
        }
    }
}
