use std::collections::HashSet;

pub struct RecoveryCertificateRevocation {

    revoked:
        HashSet<String>,
}

impl RecoveryCertificateRevocation {

    pub fn new() -> Self {

        Self {

            revoked:
                HashSet::new(),
        }
    }

    pub fn revoke(
        &mut self,
        certificate_id: String,
    ) {

        self.revoked.insert(
            certificate_id
        );
    }

    pub fn is_revoked(
        &self,
        certificate_id: &str,
    ) -> bool {

        self.revoked.contains(
            certificate_id
        )
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== REVOKED CERTIFICATES ====="
        );

        for cert
            in &self.revoked
        {

            println!(
                "{}",
                cert
            );
        }

        println!(
            "Total Revoked: {}",
            self.revoked.len()
        );
    }
}
