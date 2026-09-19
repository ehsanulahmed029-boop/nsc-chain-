use std::collections::HashMap;

pub struct LeaderElectionV4;

impl LeaderElectionV4 {

    pub fn elect(
        rankings:
            &HashMap<String, f64>,
    ) -> Option<String> {

        let mut best_validator:
            Option<String> = None;

        let mut best_score =
            0.0;

        for (validator, score)
            in rankings
        {

            if *score > best_score {

                best_score =
                    *score;

                best_validator =
                    Some(
                        validator.clone()
                    );
            }
        }

        best_validator
    }

    pub fn show(
        rankings:
            &HashMap<String, f64>,
    ) {

        println!(
            "\n===== LEADER ELECTION V4 ====="
        );

        match Self::elect(
            rankings
        ) {

            Some(leader) => {

                println!(
                    "Selected Leader: {}",
                    leader
                );
            }

            None => {

                println!(
                    "No Leader Available"
                );
            }
        }
    }
}
