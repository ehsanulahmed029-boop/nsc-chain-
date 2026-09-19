use sha2::{Digest, Sha256};

pub fn calculate_hash(data: &str) -> String {
    let mut hasher = Sha256::new();

    hasher.update(data.as_bytes());

    let result = hasher.finalize();

    format!("{:x}", result)
}
