use std::collections::HashMap;

#[derive(Debug)]
pub struct HeartbeatManager {
    pub last_seen: HashMap<String, u64>,
}

impl HeartbeatManager {

    pub fn new() -> Self {
        Self {
            last_seen: HashMap::new(),
        }
    }

    pub fn heartbeat(
        &mut self,
        validator: String,
        current_epoch: u64,
    ) {

        self.last_seen.insert(
            validator,
            current_epoch,
        );
    }

    pub fn is_online(
        &self,
        validator: &str,
        current_epoch: u64,
        max_delay: u64,
    ) -> bool {

        match self.last_seen.get(validator) {

            Some(last_epoch) => {

                current_epoch
                    <= (*last_epoch + max_delay)
            }

            None => false,
        }
    }

    pub fn offline_epochs(
        &self,
        validator: &str,
        current_epoch: u64,
    ) -> u64 {

        match self.last_seen.get(validator) {

            Some(last_epoch) => {
                current_epoch - *last_epoch
            }

            None => current_epoch,
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== HEARTBEAT STATUS ====="
        );

        for (validator, epoch)
            in &self.last_seen
        {

            println!(
                "{} => last seen epoch {}",
                validator,
                epoch
            );
        }
    }
}
