pub struct SnapshotQuorum;

impl SnapshotQuorum {

    pub fn validate(
        total_validators: usize,
        participating: usize,
        required_percent: usize,
    ) -> bool {

        if total_validators == 0 {

            return false;
        }

        let percent =
            participating * 100
                / total_validators;

        percent
            >= required_percent
    }

    pub fn show(
        total_validators: usize,
        participating: usize,
        required_percent: usize,
    ) {

        let valid =
            Self::validate(
                total_validators,
                participating,
                required_percent,
            );

        println!(
            "\n===== SNAPSHOT QUORUM ====="
        );

        println!(
            "Validators: {}",
            total_validators
        );

        println!(
            "Participating: {}",
            participating
        );

        println!(
            "Required: {}%",
            required_percent
        );

        println!(
            "Quorum Reached: {}",
            valid
        );
    }
}
