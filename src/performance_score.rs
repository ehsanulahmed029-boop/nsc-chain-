use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;
use crate::health::HealthMonitor;
use crate::uptime::UptimeTracker;

pub struct PerformanceScore;

impl PerformanceScore {

    pub fn calculate(
        validator: &str,
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        health: &HealthMonitor,
        uptime: &UptimeTracker,
    ) -> f64 {

        let stake =
            registry.validators
                .get(validator)
                .map(|v| v.stake)
                .unwrap_or(0);

        let reputation_score =
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

        (stake as f64 * 0.50)
        +
        (reputation_score as f64 * 15.0)
        +
        (health_score as f64 * 10.0)
        +
        (uptime_score * 5.0)
    }

    pub fn show_ranking(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        health: &HealthMonitor,
        uptime: &UptimeTracker,
    ) {

        let mut scores:
            Vec<(String, f64)> =
            Vec::new();

        for (validator, _)
            in &registry.validators
        {

            let score =
                Self::calculate(
                    validator,
                    registry,
                    reputation,
                    health,
                    uptime,
                );

            scores.push(
                (
                    validator.clone(),
                    score
                )
            );
        }

        scores.sort_by(
            |a, b|
            b.1.partial_cmp(&a.1)
            .unwrap()
        );

        println!(
            "\n===== PERFORMANCE RANKING ====="
        );

        for (
            rank,
            (validator, score)
        )
        in scores.iter()
            .enumerate()
        {

            println!(
                "#{} {} => {:.2}",
                rank + 1,
                validator,
                score
            );
        }
    }
}
