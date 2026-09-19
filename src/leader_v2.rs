use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;
use crate::heartbeat::HeartbeatManager;

pub struct LeaderV2;

impl LeaderV2 {

    pub fn score(
        validator: &str,
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        heartbeat: &HeartbeatManager,
        current_epoch: u64,
    ) -> f64 {

        let stake = match registry
            .validators
            .get(validator)
        {
            Some(v) => v.stake as f64,
            None => 0.0,
        };

        let reputation_score =
            reputation.score_of(
                validator
            ) as f64;

        let uptime_bonus =
            if heartbeat.is_online(
                validator,
                current_epoch,
                3,
            ) {
                100.0
            } else {
                0.0
            };

        stake
        + (reputation_score * 10.0)
        + uptime_bonus
    }

    pub fn select(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        heartbeat: &HeartbeatManager,
        current_epoch: u64,
    ) -> Option<String> {

        let mut best =
            None;

        let mut best_score =
            0.0;

        for (address, _) in
            &registry.validators
        {

            let score =
                Self::score(
                    address,
                    registry,
                    reputation,
                    heartbeat,
                    current_epoch,
                );

            if score > best_score {

                best_score =
                    score;

                best =
                    Some(
                        address.clone()
                    );
            }
        }

        best
    }

    pub fn show_scores(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        heartbeat: &HeartbeatManager,
        current_epoch: u64,
    ) {

        println!(
            "\n===== LEADER V2 SCORES ====="
        );

        for (address, _) in
            &registry.validators
        {

            let score =
                Self::score(
                    address,
                    registry,
                    reputation,
                    heartbeat,
                    current_epoch,
                );

            println!(
                "{} => {:.2}",
                address,
                score
            );
        }
    }
}
