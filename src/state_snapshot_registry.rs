use std::collections::HashMap;

use crate::state_snapshot_verify::
    StateSnapshot;

pub struct StateSnapshotRegistry {

    pub snapshots:
        HashMap<u64, StateSnapshot>,
}

impl StateSnapshotRegistry {

    pub fn new() -> Self {

        Self {
            snapshots:
                HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        snapshot: StateSnapshot,
    ) {

        self.snapshots.insert(
            snapshot.height,
            snapshot,
        );
    }

    pub fn get(
        &self,
        height: u64,
    ) -> Option<&StateSnapshot> {

        self.snapshots.get(
            &height
        )
    }

    pub fn latest(
        &self,
    ) -> Option<&StateSnapshot> {

        let max_height =
            self.snapshots
                .keys()
                .max()
                .cloned()?;

        self.snapshots.get(
            &max_height
        )
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== SNAPSHOT REGISTRY ====="
        );

        for (height, snapshot)
            in &self.snapshots
        {

            println!(
                "Height={} Hash={}",
                height,
                snapshot.state_hash
            );
        }

        println!(
            "Total Snapshots: {}",
            self.snapshots.len()
        );
    }
}
