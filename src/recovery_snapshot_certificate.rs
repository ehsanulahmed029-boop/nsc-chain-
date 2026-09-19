use sha2::{
    Digest,
    Sha256,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecoveryCertificate {

    pub height: u64,

    pub snapshot_hash: String,

    pub certificate_id: String,
}

pub struct RecoverySnapshotCertificate;

impl RecoverySnapshotCertificate {

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

    pub fn generate(
        height: u64,
        snapshot_hash: String,
    ) -> RecoveryCertificate {

        let seed =
            format!(
                "{}:{}",
                height,
                snapshot_hash
            );

        let certificate_id =
            Self::hash(
                &seed
            );

        RecoveryCertificate {

            height,

            snapshot_hash,

            certificate_id,
        }
    }

    pub fn verify(
        cert:
            &RecoveryCertificate,
    ) -> bool {

        let seed =
            format!(
                "{}:{}",
                cert.height,
                cert.snapshot_hash
            );

        let expected =
            Self::hash(
                &seed
            );

        expected
            == cert.certificate_id
    }

    pub fn show(
        cert:
            &RecoveryCertificate,
    ) {

        println!(
            "\n===== RECOVERY CERTIFICATE ====="
        );

        println!(
            "Height: {}",
            cert.height
        );

        println!(
            "Snapshot Hash: {}",
            cert.snapshot_hash
        );

        println!(
            "Certificate: {}",
            cert.certificate_id
        );
    }
}
