use crate::validator_registry::ValidatorRegistry;
use crate::reputation::ReputationManager;

pub struct ElectionAudit;

impl ElectionAudit {

    pub fn audit(
        registry: &ValidatorRegistry,
        reputation: &ReputationManager,
    ) {

        println!(
            "\n===== ELECTION AUDIT ====="
        );

        for (address, validator)
            in &registry.validators
        {

            let rep =
                reputation.reputation(
                    address
                );

            let mut risk = 0;

            if validator.stake > 100_000 {
                risk += 40;
            }

            if rep < 50 {
                risk += 40;
            }

            if validator.stake > 0
                && rep == 0
            {
                risk += 20;
            }

            println!(
                "{} => Risk Score {}",
                address,
                risk
            );

            if risk >= 60 {

                println!(
                    "ALERT: Suspicious Validator {}",
                    address
                );
            }
        }
    }
}
