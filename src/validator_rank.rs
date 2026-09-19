use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;
use crate::uptime::UptimeTracker;

#[derive(Debug)]
pub struct ValidatorRank;

impl ValidatorRank {

    pub fn score(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        uptime: &UptimeTracker,
        validator: &str,
    ) -> f64 {

        let stake =
            registry
                .validators
                .get(validator)
                .map(|v| v.stake)
                .unwrap_or(0) as f64;

        let rep =
            reputation.reputation(
                validator
            ) as f64;

        let up =
            uptime.uptime_percent(
                validator
            );

        (stake * 0.5)
        + (rep * 10.0)
        + (up * 20.0)
    }

    pub fn show_ranking(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        uptime: &UptimeTracker,
    ) {

        println!(
            "\n===== VALIDATOR RANKING ====="
        );

        for (address, _) in &registry.validators {

            let score =
                Self::score(
                    registry,
                    reputation,
                    uptime,
                    address,
                );

            println!(
                "{} => {:.2}",
                address,
                score
            );
        }
    }
}
