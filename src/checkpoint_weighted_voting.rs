use std::collections::HashMap;

pub struct CheckpointWeightedVoting {

    pub votes:
        HashMap<String, i64>,
}

impl CheckpointWeightedVoting {

    pub fn new() -> Self {

        Self {
            votes: HashMap::new(),
        }
    }

    pub fn vote(
        &mut self,
        validator: String,
        reputation: i64,
    ) {

        self.votes.insert(
            validator,
            reputation,
        );
    }

    pub fn total_weight(
        &self,
    ) -> i64 {

        self.votes
            .values()
            .sum()
    }

    pub fn validator_weight(
        &self,
        validator: &str,
    ) -> i64 {

        *self.votes
            .get(validator)
            .unwrap_or(&0)
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== WEIGHTED VOTING ====="
        );

        for (validator, weight)
            in &self.votes
        {

            println!(
                "{} => weight {}",
                validator,
                weight
            );
        }

        println!(
            "Total Weight: {}",
            self.total_weight()
        );
    }
}
