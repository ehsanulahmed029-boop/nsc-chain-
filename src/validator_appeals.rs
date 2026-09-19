use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Appeal {

    pub validator: String,

    pub reason: String,

    pub approved: bool,
}

pub struct ValidatorAppeals {

    pub appeals:
        HashMap<String, Appeal>,
}

impl ValidatorAppeals {

    pub fn new() -> Self {

        Self {
            appeals:
                HashMap::new(),
        }
    }

    pub fn submit(
        &mut self,
        validator: String,
        reason: String,
    ) {

        let appeal =
            Appeal {

                validator:
                    validator.clone(),

                reason,

                approved: false,
            };

        self.appeals.insert(
            validator,
            appeal,
        );
    }

    pub fn approve(
        &mut self,
        validator: &str,
    ) {

        if let Some(a) =
            self.appeals.get_mut(
                validator
            )
        {

            a.approved = true;
        }
    }

    pub fn approved(
        &self,
        validator: &str,
    ) -> bool {

        self.appeals
            .get(validator)
            .map(|a| a.approved)
            .unwrap_or(false)
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== VALIDATOR APPEALS ====="
        );

        for (_, appeal)
            in &self.appeals
        {

            println!(
                "{} | approved={} | reason={}",
                appeal.validator,
                appeal.approved,
                appeal.reason
            );
        }
    }
}
