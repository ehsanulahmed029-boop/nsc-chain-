#[derive(Debug, Clone)]
pub struct GovernanceState {

    pub proposal_count: u64,

    pub active_proposals: u64,

    pub passed_proposals: u64,

    pub governance_version: u32,
}

pub struct GovernanceStateSnapshot {

    pub snapshot:
        Option<GovernanceState>,
}

impl GovernanceStateSnapshot {

    pub fn new() -> Self {

        Self {
            snapshot: None,
        }
    }

    pub fn save(
        &mut self,
        proposal_count: u64,
        active_proposals: u64,
        passed_proposals: u64,
        governance_version: u32,
    ) {

        self.snapshot = Some(
            GovernanceState {

                proposal_count,

                active_proposals,

                passed_proposals,

                governance_version,
            }
        );
    }

    pub fn load(
        &self,
    ) -> Option<&GovernanceState> {

        self.snapshot.as_ref()
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== GOVERNANCE SNAPSHOT ====="
        );

        match &self.snapshot {

            Some(state) => {

                println!(
                    "Proposal Count: {}",
                    state.proposal_count
                );

                println!(
                    "Active Proposals: {}",
                    state.active_proposals
                );

                println!(
                    "Passed Proposals: {}",
                    state.passed_proposals
                );

                println!(
                    "Governance Version: {}",
                    state.governance_version
                );
            }

            None => {

                println!(
                    "No Governance Snapshot"
                );
            }
        }
    }
}
