use std::collections::HashMap;

#[derive(Debug)]
pub struct SecurityCouncilVoting {

    pub votes:
        HashMap<String, bool>,
}

impl SecurityCouncilVoting {

    pub fn new() -> Self {

        Self {
            votes:
                HashMap::new(),
        }
    }

    pub fn vote(
        &mut self,
        council_member: String,
        approve: bool,
    ) {

        self.votes.insert(
            council_member,
            approve,
        );
    }

    pub fn approve_count(
        &self,
    ) -> usize {

        self.votes
            .values()
            .filter(
                |v| **v
            )
            .count()
    }

    pub fn reject_count(
        &self,
    ) -> usize {

        self.votes
            .values()
            .filter(
                |v| !**v
            )
            .count()
    }

    pub fn passed(
        &self,
    ) -> bool {

        self.approve_count()
            >
            self.reject_count()
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== SECURITY COUNCIL VOTING ====="
        );

        println!(
            "Approve Votes: {}",
            self.approve_count()
        );

        println!(
            "Reject Votes: {}",
            self.reject_count()
        );

        println!(
            "Passed: {}",
            self.passed()
        );
    }
}
