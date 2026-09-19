use std::collections::HashMap;

// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): the only
// populate path, add_validator_strike() in main.rs, is never called from
// anywhere in the codebase. The top-level instance in main.rs was already
// underscore-prefixed (_strikes) by the original author, signalling it was
// known unused. strikes/banned are therefore permanently empty. No strike
// enforcement is currently active via this module. (Note: StrikeEngineV2 /
// add_weighted_strike() is a separate module — not covered by this audit.)
#[derive(Debug)]
pub struct StrikeSystem {
    pub strikes: HashMap<String, u32>,
    pub banned: HashMap<String, bool>,
}

impl StrikeSystem {

    pub fn new() -> Self {
        Self {
            strikes: HashMap::new(),
            banned: HashMap::new(),
        }
    }

    pub fn add_strike(
        &mut self,
        validator: String,
    ) {

        let entry =
            self.strikes
                .entry(validator.clone())
                .or_insert(0);

        *entry += 1;

        println!(
            "{} received strike {}",
            validator,
            *entry
        );

        if *entry >= 5 {

            self.banned.insert(
                validator.clone(),
                true,
            );

            println!(
                "PERMANENT BAN: {}",
                validator
            );
        }
    }

    pub fn strike_count(
        &self,
        validator: &str,
    ) -> u32 {

        *self.strikes
            .get(validator)
            .unwrap_or(&0)
    }

    pub fn is_banned(
        &self,
        validator: &str,
    ) -> bool {

        *self.banned
            .get(validator)
            .unwrap_or(&false)
    }

    pub fn penalty_level(
        &self,
        validator: &str,
    ) -> &'static str {

        let count =
            self.strike_count(
                validator
            );

        match count {

            0 => "NORMAL",

            1 => "WARNING",

            2 => "REWARD_REDUCTION",

            3 | 4 => "TEMP_JAIL",

            _ => "PERMANENT_BAN",
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== STRIKE REPORT ====="
        );

        for (validator, strikes)
            in &self.strikes
        {

            println!(
                "{} => strikes={} level={}",
                validator,
                strikes,
                self.penalty_level(
                    validator
                )
            );
        }
    }
}
