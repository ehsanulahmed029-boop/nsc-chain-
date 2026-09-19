use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ValidatorLocation {

    pub validator: String,

    pub country: String,

    pub provider: String,
}

pub struct GeographicDiversity;

impl GeographicDiversity {

    pub fn analyze(
        validators:
            &Vec<ValidatorLocation>,
    ) {

        println!(
            "\n===== GEOGRAPHIC DIVERSITY ====="
        );

        let mut countries:
            HashMap<String, usize> =
            HashMap::new();

        let mut providers:
            HashMap<String, usize> =
            HashMap::new();

        for validator
            in validators
        {

            *countries
                .entry(
                    validator.country.clone()
                )
                .or_insert(0)
                += 1;

            *providers
                .entry(
                    validator.provider.clone()
                )
                .or_insert(0)
                += 1;
        }

        println!(
            "\nCountry Distribution:"
        );

        for (country, count)
            in &countries
        {

            println!(
                "{} => {}",
                country,
                count
            );
        }

        println!(
            "\nProvider Distribution:"
        );

        for (provider, count)
            in &providers
        {

            println!(
                "{} => {}",
                provider,
                count
            );
        }

        let total =
            validators.len();

        for (country, count)
            in &countries
        {

            let percent =
                (*count as f64
                / total as f64)
                * 100.0;

            if percent > 50.0 {

                println!(
                    "WARNING: {} controls {:.2}% of validators",
                    country,
                    percent
                );
            }
        }

        for (provider, count)
            in &providers
        {

            let percent =
                (*count as f64
                / total as f64)
                * 100.0;

            if percent > 50.0 {

                println!(
                    "WARNING: Provider {} controls {:.2}% of validators",
                    provider,
                    percent
                );
            }
        }
    }
}
