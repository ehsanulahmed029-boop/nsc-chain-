use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;
use crate::uptime::UptimeTracker;

pub struct LeaderSelection;

impl LeaderSelection {

    pub fn select(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        uptime: &UptimeTracker,
    ) -> Option<String> {

        let mut winner = None;
        let mut best_score = 0f64;

        for (address, validator)
            in &registry.validators
        {

            let stake_score =
                validator.stake as f64;

            let reputation_score =
    reputation.reputation(address) as f64;

            let uptime_score =
    uptime.uptime_percent(address) as f64;

            let final_score =
                (stake_score * 0.60)
                +
                (reputation_score * 0.25)
                +
                (uptime_score * 0.15);

            if final_score > best_score {

                best_score = final_score;

                winner =
                    Some(address.clone());
            }
        }

        winner
    }

    pub fn show(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        uptime: &UptimeTracker,
    ) {

        println!(
            "\n===== LEADER SCORES ====="
        );

        for (address, validator)
            in &registry.validators
        {

            let stake_score =
                validator.stake as f64;

            let reputation_score =
    reputation.reputation(address) as f64;

            let uptime_score =
    uptime.uptime_percent(address) as f64;

            let final_score =
                (stake_score * 0.60)
                +
                (reputation_score * 0.25)
                +
                (uptime_score * 0.15);

            println!(
                "{} => {:.2}",
                address,
                final_score
            );
        }
    }
}
