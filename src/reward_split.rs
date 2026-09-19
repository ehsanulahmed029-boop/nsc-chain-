use std::collections::HashMap;

#[derive(Debug)]
pub struct RewardSplitEngine {
    pub validator_rewards: HashMap<String, u64>,
    pub delegator_rewards: HashMap<String, u64>,
}

impl RewardSplitEngine {

    pub fn new() -> Self {
        Self {
            validator_rewards: HashMap::new(),
            delegator_rewards: HashMap::new(),
        }
    }

    pub fn distribute(
        &mut self,
        validator: String,
        delegator: String,
        reward: u64,
        commission_percent: u64,
    ) {

        let validator_cut =
            reward * commission_percent / 100;

        let delegator_cut =
            reward - validator_cut;

        let v =
            self.validator_rewards
                .entry(validator)
                .or_insert(0);

        *v += validator_cut;

        let d =
            self.delegator_rewards
                .entry(delegator)
                .or_insert(0);

        *d += delegator_cut;
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== VALIDATOR REWARDS ====="
        );

        for (v, r)
            in &self.validator_rewards
        {
            println!(
                "{} => {}",
                v,
                r
            );
        }

        println!(
            "\n===== DELEGATOR REWARDS ====="
        );

        for (d, r)
            in &self.delegator_rewards
        {
            println!(
                "{} => {}",
                d,
                r
            );
        }
    }
}
