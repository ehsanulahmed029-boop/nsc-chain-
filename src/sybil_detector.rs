use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ValidatorIdentity {

    pub address: String,

    pub ip: String,

    pub fingerprint: String,
}

// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): the only
// caller, run_sybil_detection() in main.rs, is never called from
// anywhere in the codebase. Additionally, ValidatorIdentity (the input
// type) is never instantiated anywhere, so even if wired in there is no
// real data source for it yet. No sybil detection is currently active.
pub struct SybilDetector;

impl SybilDetector {

    pub fn detect(
        validators:
            &Vec<ValidatorIdentity>,
    ) {

        println!(
            "\n===== SYBIL DETECTOR ====="
        );

        let mut ip_map:
            HashMap<String, usize> =
            HashMap::new();

        let mut fp_map:
            HashMap<String, usize> =
            HashMap::new();

        for validator
            in validators
        {

            *ip_map
                .entry(
                    validator.ip.clone()
                )
                .or_insert(0)
                += 1;

            *fp_map
                .entry(
                    validator.fingerprint.clone()
                )
                .or_insert(0)
                += 1;
        }

        for (ip, count)
            in &ip_map
        {

            if *count > 1 {

                println!(
                    "WARNING: Duplicate IP {} count={}",
                    ip,
                    count
                );
            }
        }

        for (fp, count)
            in &fp_map
        {

            if *count > 1 {

                println!(
                    "WARNING: Duplicate Fingerprint {} count={}",
                    fp,
                    count
                );
            }
        }

        println!(
            "Sybil Scan Complete"
        );
    }
}
