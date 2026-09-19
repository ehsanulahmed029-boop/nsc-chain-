use crate::reputation::ReputationManager;
use crate::uptime::UptimeTracker;
use crate::slashing::Slashing;

pub struct HealthScore;

impl HealthScore {

    pub fn calculate(
        validator: &str,
        reputation: &ReputationManager,
        uptime: &UptimeTracker,
        slashing: &Slashing,
    ) -> f64 {

        let rep_score =
            reputation.reputation(
                validator
            ) as f64;

        let uptime_score =
            uptime.uptime_percent(
                validator
            );

        let penalty =
            slashing.penalties
                .get(validator)
                .cloned()
                .unwrap_or(0) as f64;

        let health =
            (rep_score * 0.4)
            + (uptime_score * 0.6)
            - penalty;

        if health < 0.0 {
            0.0
        } else {
            health
        }
    }

    pub fn show(
        validator: &str,
        reputation: &ReputationManager,
        uptime: &UptimeTracker,
        slashing: &Slashing,
    ) {

        let score =
            Self::calculate(
                validator,
                reputation,
                uptime,
                slashing,
            );

        println!(
            "{} => Health Score {:.2}",
            validator,
            score
        );
    }
}
