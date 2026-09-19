use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;
use crate::uptime::UptimeTracker;
use crate::reward_vault::RewardVault;

pub struct EpochRewardEngine;

impl EpochRewardEngine {

    pub fn distribute(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        uptime: &UptimeTracker,
        vault: &mut RewardVault,
        epoch_reward_pool: u64,
    ) {

        let total_stake =
            registry.total_stake();

        if total_stake == 0 {
            return;
        }

        for (address, validator)
            in &registry.validators
        {

            let stake_score =
                validator.stake as f64
                / total_stake as f64;

            let rep_score =
                reputation.reputation(address)
                as f64
                / 100.0;

            let uptime_score =
                uptime.uptime_percent(address)
                / 100.0;

            let final_weight =
                (stake_score * 0.50)
                + (rep_score * 0.25)
                + (uptime_score * 0.25);

            let reward =
                (epoch_reward_pool as f64
                * final_weight)
                as u64;

            vault.add_reward(
                address.clone(),
                reward,
            );
        }
    }
}
