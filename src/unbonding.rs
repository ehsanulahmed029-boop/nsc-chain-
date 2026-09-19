use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct UnbondRequest {
    pub amount: u64,
    pub unlock_epoch: u64,
}

#[derive(Debug)]
pub struct UnbondingManager {
    pub requests:
        HashMap<String, Vec<UnbondRequest>>,
}

impl UnbondingManager {

    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
        }
    }

    pub fn request_unbond(
        &mut self,
        validator: String,
        amount: u64,
        current_epoch: u64,
        waiting_epochs: u64,
    ) {

        let unlock_epoch =
            current_epoch
            + waiting_epochs;

        self.requests
            .entry(validator)
            .or_insert(Vec::new())
            .push(
                UnbondRequest {
                    amount,
                    unlock_epoch,
                }
            );
    }

    pub fn claimable(
        &self,
        validator: &str,
        current_epoch: u64,
    ) -> u64 {

        match self.requests.get(validator) {

            Some(records) => {

                records
                    .iter()
                    .filter(
                        |r|
                        r.unlock_epoch
                        <= current_epoch
                    )
                    .map(
                        |r|
                        r.amount
                    )
                    .sum()
            }

            None => 0,
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== UNBONDING QUEUE ====="
        );

        for (validator, records)
            in &self.requests
        {

            println!(
                "\nValidator: {}",
                validator
            );

            for r in records {

                println!(
                    "amount={} unlock_epoch={}",
                    r.amount,
                    r.unlock_epoch
                );
            }
        }
    }
}
