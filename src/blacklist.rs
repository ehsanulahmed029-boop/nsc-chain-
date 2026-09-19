use std::collections::HashSet;

#[derive(Debug)]
pub struct BlacklistRegistry {

    pub validators:
        HashSet<String>,
}

impl BlacklistRegistry {

    pub fn new() -> Self {

        Self {
            validators:
                HashSet::new(),
        }
    }

    pub fn ban(
        &mut self,
        validator: String,
    ) {

        self.validators.insert(
            validator.clone()
        );

        println!(
            "PERMANENTLY BANNED: {}",
            validator
        );
    }

    pub fn unban(
        &mut self,
        validator: &str,
    ) {

        self.validators.remove(
            validator
        );

        println!(
            "REMOVED FROM BLACKLIST: {}",
            validator
        );
    }

    pub fn is_banned(
        &self,
        validator: &str,
    ) -> bool {

        self.validators.contains(
            validator
        )
    }

    pub fn total_banned(
        &self,
    ) -> usize {

        self.validators.len()
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== BLACKLIST ====="
        );

        for validator
            in &self.validators
        {

            println!(
                "{}",
                validator
            );
        }
    }
}
