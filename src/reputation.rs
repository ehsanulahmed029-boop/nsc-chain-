use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ValidatorReputation {
    pub score: i64,
    pub successful_blocks: u64,
    pub missed_blocks: u64,
}

#[derive(Debug)]
pub struct ReputationManager {
    pub validators: HashMap<String, ValidatorReputation>,
}

impl ReputationManager {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        validator: String,
    ) {
        self.validators.insert(
            validator,
            ValidatorReputation {
                score: 100,
                successful_blocks: 0,
                missed_blocks: 0,
            },
        );
    }

pub fn score_of(
    &self,
    validator: &str,
) -> i32 {

    self.validators
        .get(validator)
        .map(|v| v.score as i32)
        .unwrap_or(100)
}

    pub fn reward_success(
        &mut self,
        validator: &str,
    ) {
        if let Some(v) =
            self.validators.get_mut(validator)
        {
            v.successful_blocks += 1;
            v.score += 2;
        }
    }

    pub fn punish_miss(
        &mut self,
        validator: &str,
    ) {
        if let Some(v) =
            self.validators.get_mut(validator)
        {
            v.missed_blocks += 1;
            v.score -= 5;
        }
    }

    // Compatibility method
    pub fn reward(
        &mut self,
        validator: &str,
        points: i32,
    ) {
        if let Some(v) =
            self.validators.get_mut(validator)
        {
            v.score += points as i64;
        }
    }

    // Compatibility method
    pub fn punish(
        &mut self,
        validator: &str,
        points: i32,
    ) {
        if let Some(v) =
            self.validators.get_mut(validator)
        {
            v.score -= points as i64;

            if v.score < 0 {
                v.score = 0;
            }
        }
    }

    pub fn reputation(
        &self,
        validator: &str,
    ) -> i64 {
        self.validators
            .get(validator)
            .map(|v| v.score)
            .unwrap_or(0)
    }

    pub fn is_trusted(
        &self,
        validator: &str,
    ) -> bool {
        self.reputation(validator) >= 80
    }

    pub fn show(
        &self,
    ) {
        println!(
            "\n===== VALIDATOR REPUTATION ====="
        );

        for (addr, rep)
            in &self.validators
        {
            println!(
                "{} | score={} | success={} | missed={}",
                addr,
                rep.score,
                rep.successful_blocks,
                rep.missed_blocks
            );
        }
    }
}
