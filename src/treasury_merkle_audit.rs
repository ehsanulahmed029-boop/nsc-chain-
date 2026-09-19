use sha2::{Digest, Sha256};

// WARNING: UNUSED (found 2026-08-09). merkle_root() logic is
// correct (real SHA256 merkle tree), but the only caller,
// compute_treasury_merkle_root() in main.rs, is itself never
// called anywhere. Do not assume any merkle audit of treasury
// records is actually happening.
pub struct TreasuryMerkleAudit;

impl TreasuryMerkleAudit {

    fn hash(data: &str) -> String {

        let mut hasher =
            Sha256::new();

        hasher.update(
            data.as_bytes()
        );

        format!(
            "{:x}",
            hasher.finalize()
        )
    }

    pub fn merkle_root(
        records: &[String],
    ) -> String {

        if records.is_empty() {

            return "EMPTY".to_string();
        }

        let mut hashes:
            Vec<String> =
            records.iter()
            .map(|r|
                Self::hash(r)
            )
            .collect();

        while hashes.len() > 1 {

            let mut next =
                Vec::new();

            let mut i = 0;

            while i < hashes.len() {

                let left =
                    &hashes[i];

                let right =
                    if i + 1
                        < hashes.len()
                    {
                        &hashes[i + 1]
                    } else {
                        &hashes[i]
                    };

                let combined =
                    format!(
                        "{}{}",
                        left,
                        right
                    );

                next.push(
                    Self::hash(
                        &combined
                    )
                );

                i += 2;
            }

            hashes = next;
        }

        hashes[0].clone()
    }
}
