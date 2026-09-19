use crate::reputation::ReputationManager;
use crate::heartbeat::HeartbeatManager;
use crate::performance_history::PerformanceHistory;

pub struct TrustScore;

impl TrustScore {

    pub fn calculate(
        validator: &str,
        reputation: &ReputationManager,
        heartbeat: &HeartbeatManager,
        history: &PerformanceHistory,
        current_epoch: u64,
    ) -> f64 {

        let reputation_score =
            reputation.score_of(
                validator
            ) as f64;

        let performance_score =
            history.average_score(
                validator
            );

        let uptime_score =
            if heartbeat.is_online(
                validator,
                current_epoch,
                3,
            ) {
                100.0
            } else {
                50.0
            };

        (
            reputation_score * 0.40
        )
        +
        (
            performance_score * 0.40
        )
        +
        (
            uptime_score * 0.20
        )
    }

    pub fn show(
        validator: &str,
        reputation: &ReputationManager,
        heartbeat: &HeartbeatManager,
        history: &PerformanceHistory,
        current_epoch: u64,
    ) {

        let trust =
            Self::calculate(
                validator,
                reputation,
                heartbeat,
                history,
                current_epoch,
            );

        println!(
            "{} => Trust Score {:.2}",
            validator,
            trust
        );
    }
}
