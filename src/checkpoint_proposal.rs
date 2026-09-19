#[derive(Debug, Clone)]
pub struct CheckpointProposal {

    pub height: u64,

    pub proposer: String,

    pub block_hash: String,
}

pub struct CheckpointProposalEngine {

    pub proposals:
        Vec<CheckpointProposal>,
}

impl CheckpointProposalEngine {

    pub fn new() -> Self {

        Self {
            proposals: Vec::new(),
        }
    }

    pub fn submit(
        &mut self,
        height: u64,
        proposer: String,
        block_hash: String,
    ) {

        self.proposals.push(
            CheckpointProposal {

                height,

                proposer,

                block_hash,
            }
        );
    }

    pub fn total(
        &self,
    ) -> usize {

        self.proposals.len()
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== CHECKPOINT PROPOSALS ====="
        );

        for proposal
            in &self.proposals
        {

            println!(
                "height={} proposer={} hash={}",
                proposal.height,
                proposal.proposer,
                proposal.block_hash
            );
        }

        println!(
            "Total Proposals: {}",
            self.proposals.len()
        );
    }
}
