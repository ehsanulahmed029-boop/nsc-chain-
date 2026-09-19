use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VestingReward {
    pub total_amount: u64,
    pub claimed_amount: u64,
    pub unlock_epoch: u64,
}

#[derive(Debug)]
pub struct VestingManager {
    pub rewards: HashMap<String, Vec<VestingReward>>,
}

impl VestingManager {
    pub fn new() -> Self {
        Self {
            rewards: HashMap::new(),
        }
    }

    pub fn add_reward(
        &mut self,
        validator: String,
        amount: u64,
        unlock_epoch: u64,
    ) {
        self.rewards
            .entry(validator)
            .or_insert(Vec::new())
            .push(
                VestingReward {
                    total_amount: amount,
                    claimed_amount: 0,
                    unlock_epoch,
                }
            );
    }

    pub fn claimable(
        &self,
        validator: &str,
        current_epoch: u64,
    ) -> u64 {

        let mut total = 0;

        if let Some(list) =
            self.rewards.get(validator)
        {
            for reward in list {

                if current_epoch
                    >= reward.unlock_epoch
                {
                    total += reward.total_amount
                        - reward.claimed_amount;
                }
            }
        }

        total
    }

    pub fn claim(
        &mut self,
        validator: &str,
        current_epoch: u64,
    ) -> u64 {

        let mut claimed = 0;

        if let Some(list) =
            self.rewards.get_mut(validator)
        {
            for reward in list {

                if current_epoch
                    >= reward.unlock_epoch
                {
                    let amount =
                        reward.total_amount
                        - reward.claimed_amount;

                    reward.claimed_amount += amount;

                    claimed += amount;
                }
            }
        }

        claimed
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== REWARD VESTING ====="
        );

        for (validator, rewards)
            in &self.rewards
        {
            println!(
                "\nValidator: {}",
                validator
            );

            for reward in rewards {

                println!(
                    "amount={} claimed={} unlock_epoch={}",
                    reward.total_amount,
                    reward.claimed_amount,
                    reward.unlock_epoch
                );
            }
        }
    }
}
