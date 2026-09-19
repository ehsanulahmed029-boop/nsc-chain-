use std::collections::HashMap;

#[derive(Debug)]
pub struct ReliabilityRanking;

impl ReliabilityRanking {

    pub fn calculate(
        reputation: i64,
        availability: f64,
        uptime: f64,
        performance: f64,
    ) -> f64 {

        let reputation_weight =
            reputation as f64 * 0.30;

        let availability_weight =
            availability * 0.30;

        let uptime_weight =
            uptime * 0.20;

        let performance_weight =
            performance * 0.20;

        reputation_weight
            +
            availability_weight
            +
            uptime_weight
            +
            performance_weight
    }

    pub fn show(
        rankings:
            &HashMap<String, f64>,
    ) {

        println!(
            "\n===== RELIABILITY RANKING ====="
        );

        let mut list =
            rankings
                .iter()
                .collect::<Vec<_>>();

        list.sort_by(
            |a, b|
            b.1.partial_cmp(a.1)
                .unwrap()
        );

        for (rank, (validator, score))
            in list.iter().enumerate()
        {

            println!(
                "#{} {} => {:.2}",
                rank + 1,
                validator,
                score
            );
        }
    }
}
