use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PerformanceRecord {
    pub epoch: u64,
    pub score: f64,
}

#[derive(Debug)]
pub struct PerformanceHistory {
    pub records:
        HashMap<String, Vec<PerformanceRecord>>,
}

impl PerformanceHistory {

    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    pub fn add_record(
        &mut self,
        validator: String,
        epoch: u64,
        score: f64,
    ) {

        self.records
            .entry(validator)
            .or_insert(Vec::new())
            .push(
                PerformanceRecord {
                    epoch,
                    score,
                }
            );
    }

    pub fn average_score(
        &self,
        validator: &str,
    ) -> f64 {

        match self.records.get(validator) {

            Some(history) => {

                if history.is_empty() {
                    return 0.0;
                }

                let total: f64 =
                    history
                        .iter()
                        .map(|r| r.score)
                        .sum();

                total
                    / history.len() as f64
            }

            None => 0.0,
        }
    }

    pub fn show_validator(
        &self,
        validator: &str,
    ) {

        println!(
            "\n===== PERFORMANCE HISTORY ====="
        );

        if let Some(records) =
            self.records.get(validator)
        {

            for r in records {

                println!(
                    "epoch={} score={:.2}",
                    r.epoch,
                    r.score
                );
            }
        }
    }

    pub fn ranking(
        &self,
    ) {

        println!(
            "\n===== HISTORICAL RANKING ====="
        );

        let mut ranking:
            Vec<(String, f64)> =
            self.records
                .keys()
                .map(|v|
                    (
                        v.clone(),
                        self.average_score(v)
                    )
                )
                .collect();

        ranking.sort_by(
            |a, b|
            b.1.partial_cmp(&a.1)
                .unwrap()
        );

        for (validator, score)
            in ranking
        {

            println!(
                "{} => {:.2}",
                validator,
                score
            );
        }
    }
}
