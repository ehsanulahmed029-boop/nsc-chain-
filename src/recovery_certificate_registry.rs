use std::collections::HashMap;

use crate::recovery_snapshot_certificate::
    RecoveryCertificate;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RecoveryCertificateRegistry {

    pub certificates:
        HashMap<String, RecoveryCertificate>,
}

impl RecoveryCertificateRegistry {

    pub fn new() -> Self {

        Self {
            certificates:
                HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        cert: RecoveryCertificate,
    ) {

        self.certificates.insert(
            cert.certificate_id.clone(),
            cert,
        );
    }

    pub fn get(
        &self,
        certificate_id: &str,
    ) -> Option<&RecoveryCertificate> {

        self.certificates.get(
            certificate_id
        )
    }

    pub fn exists(
        &self,
        certificate_id: &str,
    ) -> bool {

        self.certificates.contains_key(
            certificate_id
        )
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== CERTIFICATE REGISTRY ====="
        );

        for (id, cert)
            in &self.certificates
        {

            println!(
                "ID={} Height={}",
                id,
                cert.height
            );
        }

        println!(
            "Total Certificates: {}",
            self.certificates.len()
        );
    }
}
