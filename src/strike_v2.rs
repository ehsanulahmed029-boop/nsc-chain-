
use std::collections::HashMap;

#[derive(Debug)]
pub struct StrikeEngineV2 {

    pub strikes:
        HashMap<String, u32>,
}

impl StrikeEngineV2 {

    pub fn new() -> Self {

        Self {
            strikes:
                HashMap::new(),
        }
    }

    pub fn add_strike(
        &mut self,
        validator: &str,
        amount: u32,
    ) {

        let current =
            self.strikes
                .entry(
                    validator.to_string()
                )
                .or_insert(0);

        *current += amount;

        println!(
            "{} received {} strike(s)",
            validator,
            amount
        );
    }

    pub fn strike_count(
        &self,
        validator: &str,
    ) -> u32 {

        *self.strikes
            .get(validator)
            .unwrap_or(&0)
    }

    pub fn suspended(
        &self,
        validator: &str,
    ) -> bool {

        self.strike_count(
            validator
        ) >= 10
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== STRIKE ENGINE V2 ====="
        );

        for (validator, strikes)
            in &self.strikes
        {

            println!(
                "{} => {} strikes",
                validator,
                strikes
            );

            if *strikes >= 10 {

                println!(
                    "SUSPENDED: {}",
                    validator
                );
            }
        }
    }
}
