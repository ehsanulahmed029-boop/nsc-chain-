// WARNING: UNUSED BY DESIGN (2026-08-09). Checkpoint recovery
// certificates intentionally never expire. Not wired in.
use std::collections::HashMap;

pub struct RecoveryCertificateExpiration {

    expirations:
        HashMap<String, u64>,
}

impl RecoveryCertificateExpiration {

    pub fn new() -> Self {

        Self {

            expirations:
                HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        certificate_id: String,
        expire_epoch: u64,
    ) {

        self.expirations.insert(
            certificate_id,
            expire_epoch,
        );
    }

    pub fn is_expired(
        &self,
        certificate_id: &str,
        current_epoch: u64,
    ) -> bool {

        match self.expirations.get(
            certificate_id
        ) {

            Some(expire_epoch) => {

                current_epoch
                    >= *expire_epoch
            }

            None => true,
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== CERTIFICATE EXPIRATION ====="
        );

        for (id, epoch)
            in &self.expirations
        {

            println!(
                "{} -> {}",
                id,
                epoch
            );
        }

        println!(
            "Tracked Certificates: {}",
            self.expirations.len()
        );
    }
}
