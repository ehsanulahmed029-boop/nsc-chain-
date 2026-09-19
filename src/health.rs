use std::collections::HashMap;

#[derive(Debug)]
pub struct HealthMonitor {
    pub scores: HashMap<String, i32>,
}

impl HealthMonitor {

    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        validator: String,
    ) {
        self.scores.insert(
            validator,
            100,
        );
    }

    pub fn reward(
        &mut self,
        validator: &str,
    ) {

        if let Some(score) =
            self.scores.get_mut(validator)
        {
            *score += 1;

            if *score > 100 {
                *score = 100;
            }
        }
    }

    pub fn punish(
        &mut self,
        validator: &str,
    ) {

        if let Some(score) =
            self.scores.get_mut(validator)
        {
            *score -= 10;

            if *score < 0 {
                *score = 0;
            }
        }
    }

    pub fn health(
        &self,
        validator: &str,
    ) -> i32 {

        *self.scores
            .get(validator)
            .unwrap_or(&0)
    }

    pub fn unhealthy(
        &self,
        validator: &str,
    ) -> bool {

        self.health(
            validator
        ) < 50
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== VALIDATOR HEALTH ====="
        );

        for (v, score)
            in &self.scores
        {
            println!(
                "{} => {}",
                v,
                score
            );
        }
    }
}
