use std::collections::HashMap;

#[derive(Debug)]
pub struct CommitteeVote {

    pub votes: HashMap<String, bool>,

}

impl CommitteeVote {

    pub fn new() -> Self {

        Self {
            votes: HashMap::new(),
        }

    }

    pub fn vote(
        &mut self,
        validator: String,
        approve: bool,
    ) {

        self.votes.insert(
            validator,
            approve,
        );

    }

    pub fn approvals(
        &self,
    ) -> usize {

        self.votes
            .values()
            .filter(|&&v| v)
            .count()

    }

    pub fn rejections(
        &self,
    ) -> usize {

        self.votes
            .values()
            .filter(|&&v| !v)
            .count()

    }

    pub fn quorum_reached(
        &self,
        quorum: usize,
    ) -> bool {

        self.votes.len() >= quorum

    }

    pub fn passed(
        &self,
        quorum: usize,
    ) -> bool {

        self.quorum_reached(quorum)
        &&
        self.approvals()
        >
        self.rejections()

    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== COMMITTEE VOTES ====="
        );

        for (validator, vote)
            in &self.votes
        {

            println!(
                "{} => {}",
                validator,
                if *vote {
                    "APPROVE"
                } else {
                    "REJECT"
                }
            );

        }

    }
}
