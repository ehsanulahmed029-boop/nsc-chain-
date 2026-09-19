use std::collections::HashMap;

use crate::delegation_registry::DelegationRegistry;

#[derive(Debug)]
pub struct DelegatorRewardEngine {
    pub rewards: HashMap<String, u64>,
}

impl DelegatorRewardEngine {

    pub fn new() -> Self {
        Self {
            rewards: HashMap::new(),
        }
    }

    pub fn distribute(
        &mut self,
        registry: &DelegationRegistry,
        validator: &str,
        total_reward: u64,
        validator_commission: u64,
    ) {

        let validator_cut =
            total_reward
            * validator_commission
            / 100;

        let delegator_pool =
            total_reward
            - validator_cut;

        let total_stake =
            registry.total_delegated(
                validator
            );

        if total_stake == 0 {
            return;
        }

        if let Some(records) =
            registry.delegations.get(
                validator
            )
        {

            for record in records {

                let reward =
                    delegator_pool
                    * record.amount
                    / total_stake;

                let entry =
                    self.rewards
                        .entry(
                            record.delegator.clone()
                        )
                        .or_insert(0);

                *entry += reward;
            }
        }
    }

    pub fn reward_of(
        &self,
        delegator: &str,
    ) -> u64 {

        *self.rewards
            .get(delegator)
            .unwrap_or(&0)
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== DELEGATOR REWARDS ====="
        );

        for (delegator, reward)
            in &self.rewards
        {

            println!(
                "{} => {}",
                delegator,
                reward
            );
        }
    }
}
