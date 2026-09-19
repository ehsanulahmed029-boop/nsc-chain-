use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AvailabilityRecord {

    pub successful_checks: u64,

    pub failed_checks: u64,
}

pub struct AvailabilityMonitor {

    pub validators:
        HashMap<String, AvailabilityRecord>,
}

impl AvailabilityMonitor {

    pub fn new() -> Self {

        Self {
            validators:
                HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        validator: String,
    ) {

        self.validators.insert(
            validator,
            AvailabilityRecord {
                successful_checks: 0,
                failed_checks: 0,
            },
        );
    }

    pub fn success(
        &mut self,
        validator: &str,
    ) {

        if let Some(v) =
            self.validators.get_mut(
                validator
            )
        {
            v.successful_checks += 1;
        }
    }

    pub fn failure(
        &mut self,
        validator: &str,
    ) {

        if let Some(v) =
            self.validators.get_mut(
                validator
            )
        {
            v.failed_checks += 1;
        }
    }

    pub fn score(
        &self,
        validator: &str,
    ) -> f64 {

        match self.validators.get(
            validator
        ) {

            Some(v) => {

                let total =
                    v.successful_checks
                    +
                    v.failed_checks;

                if total == 0 {

                    return 100.0;
                }

                (
                    v.successful_checks
                    as f64
                    /
                    total as f64
                ) * 100.0
            }

            None => 0.0,
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== AVAILABILITY ====="
        );

        for (validator, data)
            in &self.validators
        {

            let total =
                data.successful_checks
                +
                data.failed_checks;

            let score =
                if total == 0 {
                    100.0
                } else {

                    (
                        data.successful_checks
                        as f64
                        /
                        total as f64
                    ) * 100.0
                };

            println!(
                "{} => {:.2}% availability",
                validator,
                score
            );
        }
    }
}
