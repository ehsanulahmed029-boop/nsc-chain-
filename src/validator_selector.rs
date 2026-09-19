use crate::validator_registry::ValidatorRegistry;

#[derive(Debug)]
pub struct ValidatorSelector;

impl ValidatorSelector {

pub fn select(
    registry: &ValidatorRegistry,
    seed: u64,
) -> Option<String> {

    let mut total_stake = 0u64;

    for (_, validator)
        in &registry.validators
    {
        if !validator.jailed {
            total_stake += validator.stake;
        }
    }

    if total_stake == 0 {
        return None;
    }

    let target =
        seed % total_stake;

    let mut cumulative = 0u64;

    for (address, validator)
        in &registry.validators
    {
        if validator.jailed {
            continue;
        }

        cumulative += validator.stake;

        if cumulative > target {
            return Some(
                address.clone()
            );
        }
    }

    None
}

pub fn show_pool(
    registry: &ValidatorRegistry,
) {

    println!(
        "\n===== VALIDATOR POOL ====="
    );

    for (address, validator)
        in &registry.validators
    {
        println!(
            "{} | stake={} | jailed={}",
            address,
            validator.stake,
            validator.jailed
        );
    }
}

}
