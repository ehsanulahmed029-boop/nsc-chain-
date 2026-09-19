use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HostingNode {

    pub validator: String,

    pub provider: String,
}

pub struct HostingRiskAnalyzer;

impl HostingRiskAnalyzer {

    pub fn analyze(
        nodes: &Vec<HostingNode>,
    ) {

        println!(
            "\n===== HOSTING RISK ANALYZER ====="
        );

        let mut providers:
            HashMap<String, usize> =
            HashMap::new();

        for node in nodes {

            *providers
                .entry(
                    node.provider.clone()
                )
                .or_insert(0)
                += 1;
        }

        let total =
            nodes.len();

        let mut risk_score =
            0.0;

        for (provider, count)
            in &providers
        {

            let percent =
                (*count as f64
                / total as f64)
                * 100.0;

            println!(
                "{} => {:.2}%",
                provider,
                percent
            );

            if percent > 50.0 {

                println!(
                    "WARNING: High dependency on {}",
                    provider
                );

                risk_score += 40.0;
            }
            else if percent > 30.0 {

                risk_score += 20.0;
            }
        }

        if risk_score > 100.0 {
            risk_score = 100.0;
        }

        println!(
            "\nInfrastructure Risk Score: {:.2}/100",
            risk_score
        );

        if risk_score >= 70.0 {

            println!(
                "CRITICAL RISK"
            );
        }
        else if risk_score >= 40.0 {

            println!(
                "MEDIUM RISK"
            );
        }
        else {

            println!(
                "LOW RISK"
            );
        }
    }
}
