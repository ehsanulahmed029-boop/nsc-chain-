use crate::blacklist::BlacklistRegistry;
use crate::reputation::ReputationManager;

pub struct AdmissionController;

impl AdmissionController {

    pub fn can_join(
        validator: &str,
        stake: u64,
        minimum_stake: u64,
        reputation: &ReputationManager,
        blacklist: &BlacklistRegistry,
    ) -> bool {

        if blacklist.is_banned(
            validator
        ) {

            println!(
                "{} rejected (blacklisted)",
                validator
            );

            return false;
        }

        if stake < minimum_stake {

            println!(
                "{} rejected (low stake)",
                validator
            );

            return false;
        }

        if reputation.score_of(
            validator
        ) < 50 {

            println!(
                "{} rejected (low reputation)",
                validator
            );

            return false;
        }

        println!(
            "{} approved",
            validator
        );

        true
    }
}
