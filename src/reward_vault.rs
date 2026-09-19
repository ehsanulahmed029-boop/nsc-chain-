use std::collections::HashMap;

#[derive(Debug)]
pub struct RewardVault {
    pub treasury_balance: u64,
    pub pending_rewards: HashMap<String, u64>,
    pub paid_rewards: HashMap<String, u64>,
}

impl RewardVault {

    pub fn new() -> Self {
        Self {
            treasury_balance: 0,
            pending_rewards: HashMap::new(),
            paid_rewards: HashMap::new(),
        }
    }

    pub fn deposit(
        &mut self,
        amount: u64,
    ) {
        self.treasury_balance += amount;
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

    pub fn payout(
        &mut self,
        validator: &str,
    ) -> bool {

        let reward =
            self.pending_rewards
                .get(validator)
                .copied()
                .unwrap_or(0);

        if reward == 0 {
            return false;
        }

        if self.treasury_balance < reward {
            return false;
        }

        self.treasury_balance -= reward;

        self.pending_rewards.remove(
            validator
        );

        let paid =
            self.paid_rewards
                .entry(
                    validator.to_string()
                )
                .or_insert(0);

        *paid += reward;

        true
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== REWARD VAULT ====="
        );

        println!(
            "Treasury Balance: {}",
            self.treasury_balance
        );

        println!(
            "\nPending Rewards:"
        );

        for (v, r)
            in &self.pending_rewards
        {
            println!(
                "{} => {}",
                v,
                r
            );
        }

        println!(
            "\nPaid Rewards:"
        );

        for (v, r)
            in &self.paid_rewards
        {
            println!(
                "{} => {}",
                v,
                r
            );
        }
    }
}
