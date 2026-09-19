use sha2::{
    Digest,
    Sha256,
};

// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): the only
// caller, generate_node_fingerprint() in main.rs, is never called from
// anywhere in the codebase. No node fingerprinting is currently active.
pub struct NodeFingerprint;

impl NodeFingerprint {

    pub fn generate(
        validator: &str,
        ip: &str,
        public_key: &str,
    ) -> String {

        let mut hasher =
            Sha256::new();

        hasher.update(
            validator.as_bytes()
        );

        hasher.update(
            ip.as_bytes()
        );

        hasher.update(
            public_key.as_bytes()
        );

        format!(
            "{:x}",
            hasher.finalize()
        )
    }

    pub fn short(
        fingerprint: &str,
    ) -> String {

        fingerprint
            .chars()
            .take(16)
            .collect()
    }
}
