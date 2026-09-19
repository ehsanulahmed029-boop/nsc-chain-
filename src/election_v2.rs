use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;
use crate::health::HealthMonitor;
use crate::rotation::ValidatorRotation;

pub struct ElectionV2;

impl ElectionV2 {

    pub fn score(
        validator: &str,
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        health: &HealthMonitor,
        rotation: &ValidatorRotation,
    ) -> f64 {

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

        let rotation_bonus =
            rotation.rounds_since_selected(
                validator
            );

        (stake as f64 * 0.40)
        +
        (rep as f64 * 20.0)
        +
        (health_score as f64 * 10.0)
        +
        (rotation_bonus as f64 * 25.0)
    }

    pub fn elect(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
        health: &HealthMonitor,
        rotation: &ValidatorRotation,
    ) -> Option<String> {

        let mut best_validator = None;
        let mut best_score = 0.0;

        for (address, _)
            in &registry.validators
        {

            let score =
                Self::score(
                    address,
                    registry,
                    reputation,
                    health,
                    rotation,
                );

            if score > best_score {

                best_score = score;

                best_validator =
                    Some(
                        address.clone()
                    );
            }
        }

        best_validator
    }
}
