use sha2::{
    Digest,
    Sha256,
};

#[derive(Debug, Clone)]
// ⚠️ DEAD-BY-DESIGN (Batch 2 audit, 2026-08-10): This module is never
// instantiated/called anywhere in main.rs or elsewhere. Its output would be
// misleading if displayed, since it reflects no real state. Do not wire this
// up without re-auditing the logic for correctness first. See audit notes.

pub struct AuditHashRecord {

    pub data: String,

    pub previous_hash: String,

    pub hash: String,
}

pub struct TreasuryAuditHashChain {

    pub records:
        Vec<AuditHashRecord>,
}

impl TreasuryAuditHashChain {

    pub fn new() -> Self {

        Self {
            records: Vec::new(),
        }
    }

    fn calculate_hash(
        data: &str,
        previous_hash: &str,
    ) -> String {

        let mut hasher =
            Sha256::new();

        hasher.update(
            data.as_bytes()
        );

        hasher.update(
            previous_hash.as_bytes()
        );

        format!(
            "{:x}",
            hasher.finalize()
        )
    }

    pub fn add_record(
        &mut self,
        data: String,
    ) {

        let previous_hash =
            self.records
                .last()
                .map(|r| r.hash.clone())
                .unwrap_or_else(
                    || "GENESIS".to_string()
                );

        let hash =
            Self::calculate_hash(
                &data,
                &previous_hash,
            );

        self.records.push(
            AuditHashRecord {

                data,

                previous_hash,

                hash,
            }
        );
    }

    pub fn verify(
        &self,
    ) -> bool {

        for i in 1..self.records.len() {

            let expected_hash =
                Self::calculate_hash(
                    &self.records[i].data,
                    &self.records[i].previous_hash,
                );

            if expected_hash
                != self.records[i].hash
            {
                return false;
            }

            if self.records[i]
                .previous_hash
                != self.records[i - 1].hash
            {
                return false;
            }
        }

        true
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== AUDIT HASH CHAIN ====="
        );

        for (i, record)
            in self.records.iter().enumerate()
        {

            println!(
                "{} | {}",
                i,
                record.hash
            );
        }

        println!(
            "Chain Valid: {}",
            self.verify()
        );
    }
}
