use std::collections::HashMap;

#[derive(Debug)]
pub struct Slashing {
    pub penalties: HashMap<String, u64>,
}

impl Slashing {
    pub fn new() -> Self {
        Self {
            penalties: HashMap::new(),
        }
    }

pub fn is_banned(
    &self,
    validator: &str,
) -> bool {

    let penalty =
        self.penalties
            .get(validator)
            .unwrap_or(&0);

    *penalty >= 1000
}
    pub fn slash(
        &mut self,
        validator: String,
        amount: u64,
    ) {
        let entry =
            self.penalties
                .entry(validator)
                .or_insert(0);

        *entry += amount;
    }

    pub fn show_penalties(
        &self,
    ) {
        println!(
            "\n===== SLASHING ====="
        );

        for (validator, amount)
            in &self.penalties
        {
            println!(
                "{} => {}",
                validator,
                amount
            );
        }
    }

pub fn add_record(
    &mut self,
    validator: String,
    amount: u64,
) {
    self.slash(
        validator,
        amount,
    );
}

}
