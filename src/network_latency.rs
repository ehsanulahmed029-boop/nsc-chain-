#[derive(Debug, Clone)]
pub struct LatencyRecord {

    pub validator: String,

    pub latency_ms: u64,
}

pub struct NetworkLatencyAnalyzer;

impl NetworkLatencyAnalyzer {

    pub fn analyze(
        records: &Vec<LatencyRecord>,
    ) {

        println!(
            "\n===== NETWORK LATENCY ====="
        );

        if records.is_empty() {

            println!(
                "No latency records"
            );

            return;
        }

        let mut total = 0u64;

        let mut highest = 0u64;

        let mut lowest = u64::MAX;

        for record in records {

            println!(
                "{} => {} ms",
                record.validator,
                record.latency_ms
            );

            total +=
                record.latency_ms;

            if record.latency_ms > highest {

                highest =
                    record.latency_ms;
            }

            if record.latency_ms < lowest {

                lowest =
                    record.latency_ms;
            }

            if record.latency_ms > 250 {

                println!(
                    "WARNING: High latency validator {}",
                    record.validator
                );
            }
        }

        let average =
            total as f64
            / records.len() as f64;

        println!(
            "\nAverage Latency: {:.2} ms",
            average
        );

        println!(
            "Highest Latency: {} ms",
            highest
        );

        println!(
            "Lowest Latency: {} ms",
            lowest
        );

        let health_score =
            if average < 50.0 {
                100.0
            }
            else if average < 100.0 {
                90.0
            }
            else if average < 150.0 {
                75.0
            }
            else if average < 250.0 {
                60.0
            }
            else {
                40.0
            };

        println!(
            "Network Health Score: {:.2}/100",
            health_score
        );
    }
}
