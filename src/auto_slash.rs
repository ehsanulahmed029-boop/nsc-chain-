use crate::downtime::DowntimeTracker;
use crate::slashing::Slashing;

pub struct AutoSlasher;

impl AutoSlasher {

    pub fn process(
        downtime: &DowntimeTracker,
        slashing: &mut Slashing,
        validator: String,
        slash_amount: u64,
    ) {

        if downtime.should_penalize(
            &validator
        ) {

            slashing.slash(
                validator.clone(),
                slash_amount,
            );

            println!(
                "AUTO SLASH EXECUTED => {}",
                validator
            );
        }
    }
}
