use std::collections::HashMap;

#[derive(Debug)]
pub struct DowntimeTracker {
    pub missed_blocks: HashMap<String, u64>,
    pub penalty_threshold: u64,
}

impl DowntimeTracker {

    pub fn new(
        threshold: u64,
    ) -> Self {

        Self {
            missed_blocks: HashMap::new(),
            penalty_threshold: threshold,
        }
    }

    pub fn record_miss(
        &mut self,
        validator: &str,
    ) {

        let entry =
            self.missed_blocks
                .entry(
                    validator.to_string()
                )
                .or_insert(0);

        *entry += 1;
    }

    pub fn reset(
        &mut self,
        validator: &str,
    ) {

        self.missed_blocks.insert(
            validator.to_string(),
            0,
        );
    }

    pub fn misses(
        &self,
        validator: &str,
    ) -> u64 {

        *self.missed_blocks
            .get(validator)
            .unwrap_or(&0)
    }

    pub fn should_penalize(
        &self,
        validator: &str,
    ) -> bool {

        self.misses(
            validator
        ) >= self.penalty_threshold
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== DOWNTIME TRACKER ====="
        );

        for (v, misses)
            in &self.missed_blocks
        {
            println!(
                "{} => {} misses",
                v,
                misses
            );
        }
    }
}
