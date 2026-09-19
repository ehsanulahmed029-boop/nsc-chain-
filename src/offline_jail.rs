use crate::heartbeat::HeartbeatManager;
use crate::jail::JailManager;

pub struct OfflineJailEngine;

impl OfflineJailEngine {

    pub fn check_validator(
        heartbeat: &HeartbeatManager,
        jail: &mut JailManager,
        validator: &str,
        current_epoch: u64,
        max_delay: u64,
    ) {

        let online =
            heartbeat.is_online(
                validator,
                current_epoch,
                max_delay,
            );

        if !online {

            jail.jail(
                validator.to_string(),
                current_epoch,
            );

            println!(
                "AUTO-JAIL: {}",
                validator
            );
        }
    }

    pub fn check_many(
        heartbeat: &HeartbeatManager,
        jail: &mut JailManager,
        validators: Vec<String>,
        current_epoch: u64,
        max_delay: u64,
    ) {

        for validator in validators {

            Self::check_validator(
                heartbeat,
                jail,
                &validator,
                current_epoch,
                max_delay,
            );
        }
    }
}
