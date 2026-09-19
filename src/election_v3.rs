use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;
use crate::heartbeat::HeartbeatManager;
use crate::performance_history::PerformanceHistory;
use crate::trust_score::TrustScore;

pub struct ElectionV3;

impl ElectionV3 {

    pub fn elect(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        heartbeat: &HeartbeatManager,
        history: &PerformanceHistory,
        current_epoch: u64,
        seats: usize,
    ) -> Vec<(String, f64)> {

        let mut ranking =
            Vec::new();

        for (validator, _)
            in &registry.validators
        {

            let trust =
                TrustScore::calculate(
                    validator,
                    reputation,
                    heartbeat,
                    history,
                    current_epoch,
                );

            ranking.push(
                (
                    validator.clone(),
                    trust,
                )
            );
        }

        ranking.sort_by(
            |a, b|
            b.1.partial_cmp(&a.1)
                .unwrap()
        );

        ranking
            .into_iter()
            .take(seats)
            .collect()
    }

    pub fn show(
        winners: &Vec<(String, f64)>
    ) {

        println!(
            "\n===== ELECTION V3 ====="
        );

        for (validator, score)
            in winners
        {

            println!(
                "{} => {:.2}",
                validator,
                score
            );
        }
    }
}
