use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;
use crate::uptime::UptimeTracker;
use crate::slashing::Slashing;
use crate::health_score::HealthScore;

pub struct WeightedLeader;

impl WeightedLeader {

    pub fn select(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        uptime: &UptimeTracker,
        slashing: &Slashing,
    ) -> Option<String> {

        let mut best_validator = None;
        let mut best_score = 0.0;

        let total_stake =
            registry.total_stake() as f64;

        for (address, validator)
            in &registry.validators
        {

            let stake_score =
                if total_stake == 0.0 {
                    0.0
                } else {
                    (validator.stake as f64
                    / total_stake)
                    * 100.0
                };

            let rep_score =
                reputation.reputation(
                    address
                ) as f64;

            let health_score =
                HealthScore::calculate(
                    address,
                    reputation,
                    uptime,
                    slashing,
                );

            let final_score =
                (stake_score * 0.50)
                + (rep_score * 0.30)
                + (health_score * 0.20);

            println!(
                "{} => Weighted Score {:.2}",
                address,
                final_score
            );

            if final_score > best_score {

                best_score =
                    final_score;

                best_validator =
                    Some(
                        address.clone()
                    );
            }
        }

        best_validator
    }
}
