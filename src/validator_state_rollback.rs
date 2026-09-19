use crate::validator_state_snapshot::{
    ValidatorStateSnapshot,
};

pub struct ValidatorRollback;

impl ValidatorRollback {

    pub fn rollback(
        snapshot:
            &ValidatorStateSnapshot,
        validator: &str,
    ) {

        match snapshot.get(
            validator
        ) {

            Some(state) => {

                println!(
                    "\n===== ROLLBACK SUCCESS ====="
                );

                println!(
                    "Validator: {}",
                    state.address
                );

                println!(
                    "Stake: {}",
                    state.stake
                );

                println!(
                    "Reputation: {}",
                    state.reputation
                );
            }

            None => {

                println!(
                    "Snapshot not found"
                );
            }
        }
    }
}
