use crate::validator_registry::ValidatorRegistry;
use crate::treasury::Treasury;

#[derive(Debug)]
pub struct ValidatorPerformance;

impl ValidatorPerformance {
    pub fn show_dashboard(
        registry: &ValidatorRegistry,
        treasury: &Treasury,
    ) {

        println!(
            "\n=== VALIDATOR DASHBOARD ==="
        );

        for (validator, info)
            in &registry.validators
        {
            let rewards = treasury.claimed_amount(validator);

            let score =
                info.stake
                / 100
                - (info.strikes as u64 * 10);

            println!(
                "{} | Stake={} | Rewards={} | Strikes={} | Score={}",
                validator,
                info.stake,
                rewards,
                info.strikes,
                score
            );
        }
    }

    pub fn network_score(
        registry: &ValidatorRegistry,
    ) {

        let total =
            registry.validators.len();

        let jailed =
            registry
                .validators
                .values()
                .filter(|v| v.jailed)
                .count();

        println!(
            "\nNetwork Validators: {}",
            total
        );

        println!(
            "Jailed Validators: {}",
            jailed
        );
    }
}
