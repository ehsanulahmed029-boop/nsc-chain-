use crate::governance_state_snapshot::{
    GovernanceStateSnapshot,
};

pub struct GovernanceRollback;

impl GovernanceRollback {

    pub fn rollback(
        snapshot:
            &GovernanceStateSnapshot,
    ) {

        match snapshot.load() {

            Some(state) => {

                println!(
                    "\n===== GOVERNANCE ROLLBACK ====="
                );

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
                    "No Governance Snapshot Found"
                );
            }
        }
    }
}
