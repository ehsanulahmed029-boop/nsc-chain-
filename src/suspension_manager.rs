use crate::strike_v2::StrikeEngineV2;
use crate::availability::AvailabilityMonitor;
use crate::reputation::ReputationManager;

pub struct SuspensionManager;

impl SuspensionManager {

    pub fn should_suspend(
        validator: &str,
        strikes: &StrikeEngineV2,
        availability: &AvailabilityMonitor,
        reputation: &ReputationManager,
    ) -> bool {

        if strikes.suspended(
            validator
        ) {
            return true;
        }

        if availability.score(
            validator
        ) < 40.0 {

            return true;
        }

        if reputation.reputation(
            validator
        ) < 40 {

            return true;
        }

        false
    }

    pub fn audit(
        validator: &str,
        strikes: &StrikeEngineV2,
        availability: &AvailabilityMonitor,
        reputation: &ReputationManager,
    ) {

        println!(
            "\n===== SUSPENSION AUDIT ====="
        );

        if Self::should_suspend(
            validator,
            strikes,
            availability,
            reputation,
        ) {

            println!(
                "{} => SUSPENDED",
                validator
            );
        }
        else {

            println!(
                "{} => ACTIVE",
                validator
            );
        }
    }
}
