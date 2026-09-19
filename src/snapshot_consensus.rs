use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SnapshotVote {

    pub provider: String,

    pub snapshot_hash: String,
}

pub struct SnapshotConsensus;

impl SnapshotConsensus {

    pub fn select_consensus(
        votes: &[SnapshotVote],
    ) -> Option<String> {

        let mut counts:
            HashMap<String, u64>
                = HashMap::new();

        for vote in votes {

            *counts.entry(
                vote.snapshot_hash.clone()
            ).or_insert(0) += 1;
        }

        let mut best_hash =
            None;

        let mut best_votes =
            0;

        for (hash, count)
            in counts
        {

            if count
                > best_votes
            {

                best_votes =
                    count;

                best_hash =
                    Some(hash);
            }
        }

        best_hash
    }

    pub fn show(
        hash: &str,
    ) {

        println!(
            "\n===== SNAPSHOT CONSENSUS ====="
        );

        println!(
            "Consensus Snapshot: {}",
            hash
        );
    }
}
