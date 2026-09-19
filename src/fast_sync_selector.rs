use crate::state_snapshot_verify::
    StateSnapshot;

pub struct FastSyncSelector;

impl FastSyncSelector {

    pub fn select_best(
        snapshots:
            &[StateSnapshot],
    ) -> Option<StateSnapshot> {

        let mut best:
            Option<StateSnapshot>
                = None;

        for snapshot
            in snapshots
        {

            match &best {

                Some(current) => {

                    if snapshot.height
                        > current.height
                    {
                        best =
                            Some(
                                snapshot.clone()
                            );
                    }
                }

                None => {

                    best =
                        Some(
                            snapshot.clone()
                        );
                }
            }
        }

        best
    }

    pub fn show(
        snapshot:
            &StateSnapshot,
    ) {

        println!(
            "\n===== FAST SYNC SNAPSHOT ====="
        );

        println!(
            "Selected Height: {}",
            snapshot.height
        );

        println!(
            "State Hash: {}",
            snapshot.state_hash
        );
    }
}
