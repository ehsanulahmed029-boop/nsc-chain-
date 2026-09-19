use crate::validator_appeals::ValidatorAppeals;
use crate::security_council_voting::SecurityCouncilVoting;

pub struct AppealResolution;

impl AppealResolution {

    pub fn resolve(
        validator: &str,
        appeals: &ValidatorAppeals,
        voting: &SecurityCouncilVoting,
    ) -> bool {

        appeals.approved(
            validator
        )
        &&
        voting.passed()
    }

    pub fn show(
        validator: &str,
        appeals: &ValidatorAppeals,
        voting: &SecurityCouncilVoting,
    ) {

        println!(
            "\n===== APPEAL RESOLUTION ====="
        );

        let approved =
            Self::resolve(
                validator,
                appeals,
                voting,
            );

        println!(
            "Validator: {}",
            validator
        );

        println!(
            "Appeal Accepted: {}",
            approved
        );
    }
}
