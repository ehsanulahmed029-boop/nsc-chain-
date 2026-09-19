use std::collections::HashMap;

pub struct RecoveryManager {

    pub probation:
        HashMap<String, u64>,
}

impl RecoveryManager {

    pub fn new() -> Self {

        Self {
            probation:
                HashMap::new(),
        }
    }

    pub fn start_probation(
        &mut self,
        validator: String,
        epoch: u64,
    ) {

        self.probation.insert(
            validator,
            epoch,
        );
    }

    pub fn can_recover(
        &self,
        validator: &str,
        current_epoch: u64,
        reputation: i64,
        availability: f64,
    ) -> bool {

        match self.probation.get(
            validator
        ) {

            Some(start_epoch) => {

                let probation_done =
                    current_epoch
                    >=
                    start_epoch + 5;

                probation_done
                    &&
                    reputation >= 60
                    &&
                    availability >= 80.0
            }

            None => false,
        }
    }

    pub fn recover(
        &mut self,
        validator: &str,
    ) {

        self.probation.remove(
            validator
        );

        println!(
            "{} recovered",
            validator
        );
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== RECOVERY MANAGER ====="
        );

        for (validator, epoch)
            in &self.probation
        {

            println!(
                "{} probation since epoch {}",
                validator,
                epoch
            );
        }
    }
}
