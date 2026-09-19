use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ValidatorState {

    pub address: String,

    pub stake: u64,

    pub reputation: i64,
}

#[derive(Debug)]
pub struct ValidatorStateSnapshot {

    pub snapshots:
        HashMap<String, ValidatorState>,
}

impl ValidatorStateSnapshot {

    pub fn new() -> Self {

        Self {
            snapshots:
                HashMap::new(),
        }
    }

    pub fn save(
        &mut self,
        address: String,
        stake: u64,
        reputation: i64,
    ) {

        self.snapshots.insert(
            address.clone(),
            ValidatorState {

                address,

                stake,

                reputation,
            },
        );
    }

    pub fn get(
        &self,
        validator: &str,
    ) -> Option<&ValidatorState> {

        self.snapshots.get(
            validator
        )
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== VALIDATOR STATE SNAPSHOT ====="
        );

        for (_, state)
            in &self.snapshots
        {

            println!(
                "{} | stake={} | reputation={}",
                state.address,
                state.stake,
                state.reputation
            );
        }
    }
}
