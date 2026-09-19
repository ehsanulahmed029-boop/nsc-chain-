use std::collections::HashMap;

#[derive(Debug)]
pub struct SuspensionManager {
    pub suspended: HashMap<String, bool>,
}

impl SuspensionManager {

    pub fn new() -> Self {
        Self {
            suspended: HashMap::new(),
        }
    }

    pub fn suspend(
        &mut self,
        validator: String,
    ) {

        self.suspended.insert(
            validator,
            true,
        );
    }

    pub fn unsuspend(
        &mut self,
        validator: &str,
    ) {

        self.suspended.insert(
            validator.to_string(),
            false,
        );
    }

    pub fn is_suspended(
        &self,
        validator: &str,
    ) -> bool {

        *self.suspended
            .get(validator)
            .unwrap_or(&false)
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== SUSPENDED VALIDATORS ====="
        );

        for (v, status)
            in &self.suspended
        {
            println!(
                "{} => {}",
                v,
                status
            );
        }
    }
}
