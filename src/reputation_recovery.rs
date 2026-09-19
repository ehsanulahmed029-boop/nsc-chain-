use crate::reputation::ReputationManager;

pub struct ReputationRecovery;

impl ReputationRecovery {

    pub fn recover(
        reputation: &mut ReputationManager,
        validator: &str,
        points: i32,
    ) {

        reputation.reward(
            validator,
            points,
        );

        println!(
            "Reputation recovered for {} by {} points",
            validator,
            points
        );
    }

    pub fn gradual_recovery(
        reputation: &mut ReputationManager,
        validator: &str,
    ) {

        reputation.reward(
            validator,
            5,
        );

        println!(
            "{} gained gradual recovery",
            validator
        );
    }
}
