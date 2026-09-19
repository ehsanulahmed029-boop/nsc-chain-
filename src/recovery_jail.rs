use std::collections::HashMap;

use crate::jail::JailManager;

#[derive(Debug)]
pub struct RecoveryManager {
    pub recovery_epoch:
        HashMap<String, u64>,
}

impl RecoveryManager {

    pub fn new() -> Self {
        Self {
            recovery_epoch:
                HashMap::new(),
        }
    }

    pub fn schedule_recovery(
        &mut self,
        validator: String,
        current_epoch: u64,
        waiting_epochs: u64,
    ) {

        self.recovery_epoch.insert(
            validator,
            current_epoch
                + waiting_epochs,
        );
    }

    pub fn auto_unjail(
        &mut self,
        jail: &mut JailManager,
        current_epoch: u64,
    ) {

        let mut recovered =
            Vec::new();

        for (validator, unlock_epoch)
            in &self.recovery_epoch
        {

            if current_epoch
                >= *unlock_epoch
            {

                jail.unjail(
                    validator
                );

                recovered.push(
                    validator.clone()
                );

                println!(
                    "AUTO-UNJAIL: {}",
                    validator
                );
            }
        }

        for validator in recovered {

            self.recovery_epoch
                .remove(
                    &validator
                );
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== RECOVERY QUEUE ====="
        );

        for (validator, epoch)
            in &self.recovery_epoch
        {

            println!(
                "{} => unlock at epoch {}",
                validator,
                epoch
            );
        }
    }
}
