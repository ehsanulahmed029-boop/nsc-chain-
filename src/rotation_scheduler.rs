use std::collections::HashMap;

#[derive(Debug)]
pub struct RotationScheduler {

    pub last_selected:
        HashMap<String, u64>,
}

impl RotationScheduler {

    pub fn new() -> Self {

        Self {
            last_selected:
                HashMap::new(),
        }
    }

    pub fn select_next(
        &mut self,
        rankings:
            &HashMap<String, f64>,
        epoch: u64,
    ) -> Option<String> {

        let mut best_validator =
            None;

        let mut best_score =
            -1.0;

        for (validator, score)
            in rankings
        {

            let last_epoch =
                *self
                    .last_selected
                    .get(validator)
                    .unwrap_or(&0);

            let rotation_bonus =
                (epoch - last_epoch)
                    as f64;

            let final_score =
                score +
                rotation_bonus;

            if final_score >
                best_score
            {

                best_score =
                    final_score;

                best_validator =
                    Some(
                        validator.clone()
                    );
            }
        }

        if let Some(v) =
            &best_validator
        {

            self.last_selected
                .insert(
                    v.clone(),
                    epoch,
                );
        }

        best_validator
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== ROTATION STATUS ====="
        );

        for (validator, epoch)
            in &self.last_selected
        {

            println!(
                "{} => epoch {}",
                validator,
                epoch
            );
        }
    }
}
