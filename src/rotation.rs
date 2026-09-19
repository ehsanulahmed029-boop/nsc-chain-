use std::collections::HashMap;

#[derive(Debug)]
pub struct ValidatorRotation {
    pub last_selected: HashMap<String, u64>,
    pub round: u64,
}

impl ValidatorRotation {
    pub fn new() -> Self {
        Self {
            last_selected: HashMap::new(),
            round: 0,
        }
    }

    pub fn record_selection(
        &mut self,
        validator: String,
    ) {
        self.last_selected
            .insert(
                validator,
                self.round,
            );

        self.round += 1;
    }

    pub fn rounds_since_selected(
        &self,
        validator: &str,
    ) -> u64 {
        match self.last_selected.get(validator) {
            Some(last) => self.round - *last,
            None => self.round,
        }
    }

    pub fn show(
        &self,
    ) {
        println!(
            "\n===== VALIDATOR ROTATION ====="
        );

        for (v, round)
            in &self.last_selected
        {
            println!(
                "{} => last round {}",
                v,
                round
            );
        }
    }
}
