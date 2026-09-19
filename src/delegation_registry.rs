use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DelegationRecord {
    pub delegator: String,
    pub amount: u64,
}

#[derive(Debug)]
pub struct DelegationRegistry {
    pub delegations:
        HashMap<String, Vec<DelegationRecord>>,
}

impl DelegationRegistry {

    pub fn new() -> Self {
        Self {
            delegations: HashMap::new(),
        }
    }

    pub fn delegate(
        &mut self,
        validator: String,
        delegator: String,
        amount: u64,
    ) {

        self.delegations
            .entry(validator)
            .or_insert(Vec::new())
            .push(
                DelegationRecord {
                    delegator,
                    amount,
                }
            );
    }

    pub fn total_delegated(
        &self,
        validator: &str,
    ) -> u64 {

        match self.delegations.get(validator) {

            Some(records) => {
                records
                    .iter()
                    .map(|r| r.amount)
                    .sum()
            }

            None => 0,
        }
    }

    pub fn delegator_count(
        &self,
        validator: &str,
    ) -> usize {

        match self.delegations.get(validator) {
            Some(records) => records.len(),
            None => 0,
        }
    }

    pub fn show_validator(
        &self,
        validator: &str,
    ) {

        println!(
            "\n===== DELEGATIONS ====="
        );

        if let Some(records) =
            self.delegations.get(validator)
        {

            for r in records {

                println!(
                    "{} => {}",
                    r.delegator,
                    r.amount
                );
            }
        }
    }

    pub fn show_all(
        &self,
    ) {

        println!(
            "\n===== ALL DELEGATIONS ====="
        );

        for (validator, records)
            in &self.delegations
        {

            println!(
                "\nValidator: {}",
                validator
            );

            for r in records {

                println!(
                    "  {} => {}",
                    r.delegator,
                    r.amount
                );
            }
        }
    }
}
