use std::collections::HashMap;

// ⚠️ DEAD-BY-DESIGN (Batch 2 audit, 2026-08-10): This module is never
// instantiated/called anywhere in main.rs or elsewhere. Its output would be
// misleading if displayed, since it reflects no real state. Do not wire this
// up without re-auditing the logic for correctness first. See audit notes.

pub struct CheckpointSlashing {

    pub penalties:
        HashMap<String, u64>,
}

impl CheckpointSlashing {

    pub fn new() -> Self {

        Self {
            penalties:
                HashMap::new(),
        }
    }

    pub fn slash(
        &mut self,
        validator: String,
        amount: u64,
    ) {

        *self.penalties
            .entry(validator)
            .or_insert(0)
            += amount;
    }

    pub fn penalty(
        &self,
        validator: &str,
    ) -> u64 {

        *self.penalties
            .get(validator)
            .unwrap_or(&0)
    }

    pub fn detect_fake(
        &mut self,
        validator: String,
        valid: bool,
    ) {

        if !valid {

            self.slash(
                validator,
                100,
            );

            println!(
                "Fake Checkpoint Detected"
            );
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== CHECKPOINT SLASHING ====="
        );

        for (validator, amount)
            in &self.penalties
        {

            println!(
                "{} => penalty {}",
                validator,
                amount
            );
        }
    }
}
