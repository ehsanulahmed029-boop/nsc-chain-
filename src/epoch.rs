use std::collections::HashMap;
use crate::validator_registry::ValidatorRegistry;
use crate::validator_selector::ValidatorSelector;

#[derive(Debug, Clone)]
pub struct EpochManager {
    pub current_epoch: u64,
    pub reward_pool: u64,
    pub reward_history: HashMap<u64, u64>,
    pub validator_rewards: HashMap<String, u64>,
    pub current_validator: Option<String>,
    pub validator_history: Vec<String>,
}

impl EpochManager {
    pub fn new() -> Self {
        Self {
            current_epoch: 0,
            reward_pool: 0,
            reward_history: HashMap::new(),
            validator_rewards: HashMap::new(),
            current_validator: None,
            validator_history: Vec::new(),
        }
    }

    pub fn add_rewards(
        &mut self,
        amount: u64,
    ) {
        self.reward_pool += amount;
    }

    pub fn distribute_rewards(
        &mut self,
        registry: &ValidatorRegistry,
    ) {
        let total_stake =
            registry.total_stake();

        if total_stake == 0 {
            return;
        }

        for (address, validator)
            in &registry.validators
        {
            let reward =
                self.reward_pool
                * validator.stake
                / total_stake;

            let entry =
                self.validator_rewards
                    .entry(address.clone())
                    .or_insert(0);

            *entry += reward;
        }
    }

    pub fn finalize_epoch(
        &mut self,
        registry: &ValidatorRegistry,
    ) {
        self.distribute_rewards(
            registry
        );

        self.reward_history.insert(
            self.current_epoch,
            self.reward_pool,
        );

        println!(
            "Epoch {} finalized with reward pool {}",
            self.current_epoch,
            self.reward_pool
        );

        self.rotate_validator(
    registry
);
        self.current_epoch += 1;
        self.reward_pool = 0;
    }

    pub fn show_history(
        &self,
    ) {
        println!(
            "\n=== EPOCH HISTORY ==="
        );

        for (epoch, reward)
            in &self.reward_history
        {
            println!(
                "Epoch {} => {}",
                epoch,
                reward
            );
        }
    }

    pub fn show_rewards(
        &self,
    ) {
        println!(
            "\n=== VALIDATOR REWARDS ==="
        );

        for (validator, reward)
            in &self.validator_rewards
        {
            println!(
                "{} => {}",
                validator,
                reward
            );
        }
    }

pub fn rotate_validator(
    &mut self,
    registry: &ValidatorRegistry,
) {

    let seed =
        self.current_epoch + 1;

    let selected =
        ValidatorSelector::select(
            registry,
            seed,
        );

    if let Some(v) = selected.clone() {

        self.current_validator =
            Some(v.clone());

        self.validator_history.push(
            v
        );
    }
}

pub fn show_validator_history(
    &self,
) {

    println!(
        "\n=== VALIDATOR HISTORY ==="
    );

    for (i, v)
        in self.validator_history
            .iter()
            .enumerate()
    {
        println!(
            "Epoch {} => {}",
            i,
            v
        );
    }
}

}
