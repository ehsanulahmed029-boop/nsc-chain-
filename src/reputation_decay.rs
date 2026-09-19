use crate::reputation::ReputationManager;
use crate::heartbeat::HeartbeatManager;

pub struct ReputationDecay;

impl ReputationDecay {

    pub fn process(
        reputation: &mut ReputationManager,
        heartbeat: &HeartbeatManager,
        validator: &str,
        current_epoch: u64,
    ) {

        let offline_epochs =
            heartbeat.offline_epochs(
                validator,
                current_epoch,
            );

        if offline_epochs >= 10 {

            reputation.punish(
                validator,
                5,
            );

            println!(
                "{} reputation decay applied",
                validator
            );
        }

        if offline_epochs <= 1 {

            reputation.reward(
                validator,
                2,
            );

            println!(
                "{} activity bonus applied",
                validator
            );
        }
    }

    pub fn process_many(
        reputation: &mut ReputationManager,
        heartbeat: &HeartbeatManager,
        validators: Vec<String>,
        current_epoch: u64,
    ) {

        for validator in validators {

            Self::process(
                reputation,
                heartbeat,
                &validator,
                current_epoch,
            );
        }
    }
}
