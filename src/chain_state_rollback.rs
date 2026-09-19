use crate::chain_state_snapshot::{
    ChainStateSnapshot,
};

pub struct ChainStateRollback;

impl ChainStateRollback {

    pub fn rollback(
        snapshot:
            &ChainStateSnapshot,
    ) {

        match snapshot.load() {

            Some(state) => {

                println!(
                    "\n===== CHAIN ROLLBACK ====="
                );

                println!(
                    "Restored Height: {}",
                    state.block_height
                );

                println!(
                    "Restored Epoch: {}",
                    state.current_epoch
                );

                println!(
                    "Restored Governance: {}",
                    state.governance_version
                );

                println!(
                    "Restored Validators: {}",
                    state.validator_count
                );
            }

            None => {

                println!(
                    "No Chain Snapshot Found"
                );
            }
        }
    }
}
