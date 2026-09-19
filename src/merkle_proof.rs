use sha2::{Digest, Sha256};

pub struct MerkleProof;

impl MerkleProof {

    fn hash(
        data: &str,
    ) -> String {

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

    pub fn verify(
        leaf: &str,
        proof: Vec<String>,
        root: String,
    ) -> bool {

        let mut current =
            Self::hash(leaf);

        for sibling in proof {

            let combined =
                format!(
                    "{}{}",
                    current,
                    sibling
                );

            current =
                Self::hash(
                    &combined
                );
        }

        current == root
    }

    pub fn show(
        valid: bool,
    ) {

        println!(
            "\n===== MERKLE PROOF ====="
        );

        println!(
            "Valid: {}",
            valid
        );
    }
}
