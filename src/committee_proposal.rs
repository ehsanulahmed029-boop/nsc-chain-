use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Proposal {

    pub id: u64,
    pub title: String,
    pub description: String,
    pub executed: bool,

}

pub struct CommitteeProposalSystem {

    pub proposals: HashMap<u64, Proposal>,
    pub next_id: u64,

}

impl CommitteeProposalSystem {

    pub fn new() -> Self {

        Self {
            proposals: HashMap::new(),
            next_id: 1,
        }

    }

    pub fn create(
        &mut self,
        title: String,
        description: String,
    ) -> u64 {

        let id = self.next_id;

        self.next_id += 1;

        self.proposals.insert(
            id,
            Proposal {
                id,
                title,
                description,
                executed: false,
            },
        );

        id
    }

    pub fn execute(
        &mut self,
        proposal_id: u64,
    ) {

        if let Some(p) =
            self.proposals.get_mut(
                &proposal_id
            )
        {
            p.executed = true;

            println!(
                "Proposal {} executed",
                proposal_id
            );
        }

    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== COMMITTEE PROPOSALS ====="
        );

        for (_, p)
            in &self.proposals
        {

            println!(
                "[{}] {} | executed={}",
                p.id,
                p.title,
                p.executed
            );

        }

    }
}
