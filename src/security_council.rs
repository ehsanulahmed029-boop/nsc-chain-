use std::collections::HashSet;

#[derive(Debug)]
pub struct SecurityCouncil {
    pub validators:
        HashSet<String>,

    pub emergency_votes:
        HashSet<String>,
}

impl SecurityCouncil {

    pub fn new() -> Self {

        Self {
            validators:
                HashSet::new(),

            emergency_votes:
                HashSet::new(),
        }
    }

pub fn is_member(
    &self,
    validator: &str,
) -> bool {

    self.validators.contains(
        validator
    )
}

pub fn member_count(
    &self,
) -> usize {

    self.validators.len()
}

    pub fn add_validator(
        &mut self,
        validator: String,
    ) {

        self.validators.insert(
            validator
        );
    }

    pub fn vote_emergency(
        &mut self,
        validator: String,
    ) {

        if self.validators.contains(
            &validator
        ) {

            self.emergency_votes.insert(
                validator
            );
        }
    }

    pub fn quorum_reached(
        &self,
    ) -> bool {

        let total =
            self.validators.len();

        let votes =
            self.emergency_votes.len();

        votes * 3
            >= total * 2
    }

    pub fn clear_votes(
        &mut self,
    ) {

        self.emergency_votes.clear();
    }

    pub fn show_votes(
        &self,
    ) {

        println!(
            "\n===== SECURITY COUNCIL ====="
        );

        println!(
            "Votes: {}",
            self.emergency_votes.len()
        );

        println!(
            "Validators: {}",
            self.validators.len()
        );
    }
}
