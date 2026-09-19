use std::collections::HashMap;

#[derive(Debug)]
pub struct RewardClaimSystem {
    pub pending_rewards: HashMap<String, u64>,
    pub claimed_rewards: HashMap<String, u64>,
}

impl RewardClaimSystem {

    pub fn new() -> Self {
        Self {
            pending_rewards: HashMap::new(),
            claimed_rewards: HashMap::new(),
        }
    }

    pub fn add_reward(
        &mut self,
        validator: String,
        amount: u64,
    ) {

        let entry =
            self.pending_rewards
                .entry(validator)
                .or_insert(0);

        *entry += amount;
    }

    pub fn claim(
        &mut self,
        validator: &str,
    ) -> u64 {

        let reward =
            self.pending_rewards
                .remove(validator)
                .unwrap_or(0);

        let entry =
            self.claimed_rewards
                .entry(
                    validator.to_string()
                )
                .or_insert(0);

        *entry += reward;

        reward
    }

    pub fn pending(
        &self,
        validator: &str,
    ) -> u64 {

        *self.pending_rewards
            .get(validator)
            .unwrap_or(&0)
    }

    pub fn claimed(
        &self,
        validator: &str,
    ) -> u64 {

        *self.claimed_rewards
            .get(validator)
            .unwrap_or(&0)
    }

    pub fn show_pending(
        &self,
    ) {

        println!(
            "\n===== PENDING REWARDS ====="
        );

        for (v, amount)
            in &self.pending_rewards
        {

            println!(
                "{} => {}",
                v,
                amount
            );
        }
    }

    pub fn show_claimed(
        &self,
    ) {

        println!(
            "\n===== CLAIMED REWARDS ====="
        );

        for (v, amount)
            in &self.claimed_rewards
        {

            println!(
                "{} => {}",
                v,
                amount
            );
        }
    }
}
