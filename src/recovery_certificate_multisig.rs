use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct RecoveryMultiSig {

    pub certificate_id: String,

    pub signatures:
        HashSet<String>,
}

pub struct RecoveryCertificateMultiSig;

impl RecoveryCertificateMultiSig {

    pub fn new(
        certificate_id: String,
    ) -> RecoveryMultiSig {

        RecoveryMultiSig {

            certificate_id,

            signatures:
                HashSet::new(),
        }
    }

    pub fn sign(
        multisig:
            &mut RecoveryMultiSig,
        validator: String,
    ) {

        multisig.signatures.insert(
            validator
        );
    }

    pub fn verify(
        multisig:
            &RecoveryMultiSig,
        required: usize,
    ) -> bool {

        multisig.signatures.len()
            >= required
    }

    pub fn show(
        multisig:
            &RecoveryMultiSig,
    ) {

        println!(
            "\n===== RECOVERY MULTISIG ====="
        );

        println!(
            "Certificate: {}",
            multisig.certificate_id
        );

        println!(
            "Signatures: {}",
            multisig.signatures.len()
        );
    }
}
