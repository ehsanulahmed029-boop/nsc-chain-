use std::collections::HashMap;

pub struct CheckpointReputation {

    pub scores:
        HashMap<String, i64>,
}

impl CheckpointReputation {

    pub fn new() -> Self {

        Self {
            scores:
                HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        validator: String,
    ) {

        self.scores
            .entry(validator)
            .or_insert(100);
    }

    pub fn reward(
        &mut self,
        validator: &str,
        points: i64,
    ) {

        if let Some(score)
            = self.scores.get_mut(
                validator
            )
        {
            *score += points;
        }
    }

    pub fn punish(
        &mut self,
        validator: &str,
        points: i64,
    ) {

        if let Some(score)
            = self.scores.get_mut(
                validator
            )
        {
            *score -= points;

            if *score < 0 {

                *score = 0;
            }
        }
    }

    pub fn score(
        &self,
        validator: &str,
    ) -> i64 {

        *self.scores
            .get(validator)
            .unwrap_or(&0)
    }

    pub fn trusted(
        &self,
        validator: &str,
    ) -> bool {

        self.score(
            validator
        ) >= 80
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== CHECKPOINT REPUTATION ====="
        );

        for (validator, score)
            in &self.scores
        {

            println!(
                "{} => {}",
                validator,
                score
            );
        }
    }
}
