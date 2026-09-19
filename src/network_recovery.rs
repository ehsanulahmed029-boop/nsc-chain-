use crate::chain_freeze::ChainFreeze;
use crate::treasury_freeze::TreasuryFreeze;

#[derive(Debug)]
pub struct NetworkRecovery {

    pub recovery_mode: bool,
}

impl NetworkRecovery {

    pub fn new() -> Self {

        Self {
            recovery_mode: false,
        }
    }

    pub fn activate(
        &mut self,
    ) {

        self.recovery_mode = true;

        println!(
            "NETWORK RECOVERY MODE ENABLED"
        );
    }

    pub fn deactivate(
        &mut self,
    ) {

        self.recovery_mode = false;

        println!(
            "NETWORK RECOVERY MODE DISABLED"
        );
    }

    pub fn recover(
        &self,
        chain: &mut ChainFreeze,
        treasury: &mut TreasuryFreeze,
    ) {

        chain.unfreeze();

        treasury.unfreeze();

        println!(
            "NETWORK RECOVERY COMPLETED"
        );
    }

    pub fn status(
        &self,
    ) {

        println!(
            "\n===== RECOVERY STATUS ====="
        );

        println!(
            "Recovery Mode: {}",
            self.recovery_mode
        );
    }
}
