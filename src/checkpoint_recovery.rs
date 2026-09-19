use crate::network_checkpoint::{
    NetworkCheckpoint,
};

pub struct CheckpointRecovery;

impl CheckpointRecovery {

    pub fn recover(
        checkpoint:
            &NetworkCheckpoint,
    ) {

        match checkpoint.latest() {

            Some(cp) => {

                println!(
                    "\n===== CHECKPOINT RECOVERY ====="
                );

                println!(
                    "Restored Height: {}",
                    cp.height
                );

                println!(
                    "Restored Epoch: {}",
                    cp.epoch
                );

                println!(
                    "Restored Hash: {}",
                    cp.block_hash
                );

                println!(
                    "Recovery Successful"
                );
            }

            None => {

                println!(
                    "No Checkpoint Available"
                );
            }
        }
    }
}
