use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RewardRecord {
    pub epoch: u64,
    pub amount: u64,
}

#[derive(Debug)]
pub struct RewardHistory {
    pub history:
        HashMap<String, Vec<RewardRecord>>,
}

impl RewardHistory {

    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
        }
    }

    pub fn add_reward(
        &mut self,
        validator: String,
        epoch: u64,
        amount: u64,
    ) {

        self.history
            .entry(validator)
            .or_insert(Vec::new())
            .push(
                RewardRecord {
                    epoch,
                    amount,
                }
            );
    }

    pub fn total_rewards(
        &self,
        validator: &str,
    ) -> u64 {

        match self.history.get(validator) {

            Some(records) => {
                records
                    .iter()
                    .map(|r| r.amount)
                    .sum()
            }

            None => 0,
        }
    }

    pub fn validator_history(
        &self,
        validator: &str,
    ) {

        println!(
            "\n===== REWARD HISTORY ====="
        );

        if let Some(records) =
            self.history.get(validator)
        {

            for r in records {

                println!(
                    "epoch={} reward={}",
                    r.epoch,
                    r.amount
                );
            }
        }
    }

    pub fn show_totals(
        &self,
    ) {

        println!(
            "\n===== TOTAL REWARDS ====="
        );

        for (validator, records)
            in &self.history
        {

            let total:u64 =
                records
                .iter()
                .map(|r| r.amount)
                .sum();

            println!(
                "{} => {}",
                validator,
                total
            );
        }
    }
}
