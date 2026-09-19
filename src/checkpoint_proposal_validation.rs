use crate::checkpoint_proposal::
    CheckpointProposalEngine;

// ⚠️ DEAD-BY-DESIGN (Batch 2 audit, 2026-08-10): This module is never
// instantiated/called anywhere in main.rs or elsewhere. Its output would be
// misleading if displayed, since it reflects no real state. Do not wire this
// up without re-auditing the logic for correctness first. See audit notes.

pub struct CheckpointProposalValidation;

impl CheckpointProposalValidation {

    pub fn validate(
        engine:
            &CheckpointProposalEngine,
        height: u64,
        block_hash: &str,
    ) -> bool {

        if height == 0 {

            println!(
                "Invalid Height"
            );

            return false;
        }

        if block_hash.is_empty() {

            println!(
                "Empty Hash"
            );

            return false;
        }

        if block_hash.len() < 6 {

            println!(
                "Hash Too Short"
            );

            return false;
        }

        for proposal
            in &engine.proposals
        {

            if proposal.height
                == height
            {

                println!(
                    "Duplicate Height"
                );

                return false;
            }
        }

        true
    }

    pub fn show(
        valid: bool,
    ) {

        println!(
            "\n===== PROPOSAL VALIDATION ====="
        );

        println!(
            "Valid: {}",
            valid
        );
    }
}
