use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;
use crate::health::HealthMonitor;
use crate::uptime::UptimeTracker;

pub struct DynamicRewards;

impl DynamicRewards {

    pub fn calculate(
        validator: &str,
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        health: &HealthMonitor,
        uptime: &UptimeTracker,
    ) -> u64 {

        let stake =
            registry.validators
                .get(validator)
                .map(|v| v.stake)
                .unwrap_or(0);

        let rep =
            reputation.reputation(
                validator
            );

        let health_score =
            health.health(
                validator
            );

        let uptime_score =
            uptime.uptime_percent(
                validator
            );

        let reward =
            (stake as f64 * 0.001)
            +
            (rep as f64 * 2.0)
            +
            (health_score as f64 * 1.5)
            +
            uptime_score;

        reward as u64
    }

    pub fn show(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        health: &HealthMonitor,
        uptime: &UptimeTracker,
    ) {

        println!(
            "\n===== DYNAMIC REWARDS ====="
        );

        for (address, _)
            in &registry.validators
        {

            let reward =
                Self::calculate(
                    address,
                    registry,
                    reputation,
                    health,
                    uptime,
                );

            println!(
                "{} => {} NSC",
                address,
                reward
            );
        }
    }
}
