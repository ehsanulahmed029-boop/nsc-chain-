#[derive(Debug)]
pub struct SafeRestartManager {

    pub validator_check: bool,

    pub epoch_check: bool,

    pub consensus_check: bool,

    pub restarted: bool,
}

impl SafeRestartManager {

    pub fn new() -> Self {

        Self {

            validator_check: false,

            epoch_check: false,

            consensus_check: false,

            restarted: false,
        }
    }

    pub fn verify_validators(
        &mut self,
    ) {

        self.validator_check = true;
    }

    pub fn verify_epoch(
        &mut self,
    ) {

        self.epoch_check = true;
    }

    pub fn verify_consensus(
        &mut self,
    ) {

        self.consensus_check = true;
    }

    pub fn can_restart(
        &self,
    ) -> bool {

        self.validator_check
            &&
            self.epoch_check
            &&
            self.consensus_check
    }

    pub fn restart(
        &mut self,
    ) {

        if self.can_restart() {

            self.restarted = true;

            println!(
                "Network Restart Successful"
            );
        }
        else {

            println!(
                "Restart Blocked"
            );
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== SAFE RESTART ====="
        );

        println!(
            "Validator Check: {}",
            self.validator_check
        );

        println!(
            "Epoch Check: {}",
            self.epoch_check
        );

        println!(
            "Consensus Check: {}",
            self.consensus_check
        );

        println!(
            "Restarted: {}",
            self.restarted
        );
    }
}
