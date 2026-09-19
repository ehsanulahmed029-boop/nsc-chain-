// ============================================================
// NUSACOIN (NSC) — emergency_freeze_multisig.rs
// Added Batch 2.5 (2026-08-10) — emergency freeze/unfreeze multisig.
// ============================================================
// Mirrors the verified signature-checking pattern in multisig.rs
// (TreasuryMultiSig::sign()) exactly: Ed25519 signature over a
// canonical message, checked against a claimed owner address.
//
// Design (confirmed with operator 2026-08-10):
// - FREEZE: single-owner action, applied immediately (speed matters
//   in an active incident). Not tracked via this struct — the API
//   route verifies one owner signature directly and flips the flag.
// - UNFREEZE: full 3-owner multisig quorum required, PLUS sanity
//   checks (checkpoint cert validity, balance consistency) before
//   execution. This struct tracks unfreeze request signatures.
// - Reuses the same 3 owner addresses as treasury_multisig for now
//   (see design note: a compromised treasury key also compromises
//   emergency freeze under this scheme; a fully separate emergency
//   owner set could be introduced later if needed).
// ============================================================

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EmergencyFreezeRequest {
    pub id: String,
    /// "treasury" | "chain" | "all"
    pub target: String,
    /// Always "unfreeze" for requests tracked here (freeze is
    /// single-owner and doesn't go through this request flow).
    pub action: String,
    pub timestamp: u64,
}

impl EmergencyFreezeRequest {
    pub fn new(id: String, target: String, action: String, timestamp: u64) -> Self {
        Self { id, target, action, timestamp }
    }

    /// The exact message owners sign. Binding id + target + action
    /// together means a signature for one request can never be
    /// replayed against a different target or action.
    pub fn canonical_message(&self) -> String {
        format!("NSCEMERGENCY:{}:{}:{}", self.id, self.target, self.action)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EmergencyMultiSigError {
    NotAnOwner,
    InvalidSignature,
    AlreadySigned,
}

impl std::fmt::Display for EmergencyMultiSigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmergencyMultiSigError::NotAnOwner => write!(f, "signer is not a recognized emergency-freeze owner"),
            EmergencyMultiSigError::InvalidSignature => write!(f, "signature does not verify against the request"),
            EmergencyMultiSigError::AlreadySigned => write!(f, "this owner has already signed this request"),
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EmergencyFreezeMultiSig {
    pub owners: Vec<String>,
    pub required: usize,
    /// request_id -> set of distinct owner addresses who have
    /// validly signed it.
    signatures: HashMap<String, HashSet<String>>,
}

impl Default for EmergencyFreezeMultiSig {
    /// Only used as a serde fallback when loading an old full_state.json
    /// that predates this field. Falls back to the real production owner
    /// set (same 3 owners as treasury_multisig) rather than an empty/unsafe
    /// default, so a missing field can never silently produce a
    /// zero-owner or zero-required multisig.
    fn default() -> Self {
        Self::new(
            vec![
                "NSCe258f89e5fa12872".to_string(),
                "NSC8bfe30da185e00f3".to_string(),
                "NSCa06f136253f19163".to_string(),
            ],
            3,
        )
    }
}

impl EmergencyFreezeMultiSig {
    pub fn new(owners: Vec<String>, required: usize) -> Self {
        if required == 0 {
            eprintln!(
                "[EMERGENCY-MULTISIG] WARNING: required signature count is 0. \
                 This means every unfreeze request is approved with zero \
                 signatures. This is almost certainly a misconfiguration."
            );
        }
        if required > owners.len() {
            eprintln!(
                "[EMERGENCY-MULTISIG] WARNING: required ({}) exceeds owner count ({}). \
                 No unfreeze can ever be approved.",
                required, owners.len()
            );
        }
        Self { owners, required, signatures: HashMap::new() }
    }

    fn is_owner(&self, address: &str) -> bool {
        self.owners.iter().any(|o| o == address)
    }

    /// Records a signature from `signer` for `request`. Mirrors
    /// TreasuryMultiSig::sign() in multisig.rs exactly: verifies a
    /// real Ed25519 signature over canonical_message(), and checks
    /// the derived address matches the claimed signer.
    pub fn sign(
        &mut self,
        request: &EmergencyFreezeRequest,
        signer: &str,
        signature_hex: &str,
        public_key_hex: &str,
    ) -> Result<(), EmergencyMultiSigError> {
        if !self.is_owner(signer) {
            eprintln!("[EMERGENCY-MULTISIG] Rejected signature from non-owner: {}", signer);
            return Err(EmergencyMultiSigError::NotAnOwner);
        }

        use crate::wallet::Wallet;

        let message = request.canonical_message();

        let public_key = match Wallet::public_key_from_hex(public_key_hex) {
            Some(pk) => pk,
            None => {
                eprintln!(
                    "[EMERGENCY-MULTISIG] Invalid public key hex from {} on request {}.",
                    signer, request.id
                );
                return Err(EmergencyMultiSigError::InvalidSignature);
            }
        };

        if !Wallet::verify_signature(&public_key, &message, signature_hex) {
            eprintln!(
                "[EMERGENCY-MULTISIG] Signature verification FAILED for {} on request {}.",
                signer, request.id
            );
            return Err(EmergencyMultiSigError::InvalidSignature);
        }

        let derived_address = match Wallet::address_from_public_key_hex(public_key_hex) {
            Some(addr) => addr,
            None => {
                eprintln!("[EMERGENCY-MULTISIG] Could not derive address from public key for {}.", signer);
                return Err(EmergencyMultiSigError::InvalidSignature);
            }
        };

        if derived_address != signer {
            eprintln!(
                "[EMERGENCY-MULTISIG] Public key does not match claimed signer {} (derived {}).",
                signer, derived_address
            );
            return Err(EmergencyMultiSigError::InvalidSignature);
        }

        let signers = self.signatures.entry(request.id.clone()).or_insert_with(HashSet::new);

        if signers.contains(signer) {
            println!(
                "[EMERGENCY-MULTISIG] {} already signed request {} — no change ({}/{}).",
                signer, request.id, signers.len(), self.required
            );
            return Err(EmergencyMultiSigError::AlreadySigned);
        }

        signers.insert(signer.to_string());

        println!(
            "[EMERGENCY-MULTISIG] {} signed request {} ({}/{} required).",
            signer, request.id, signers.len(), self.required
        );

        Ok(())
    }

    pub fn is_approved(&self, request_id: &str) -> bool {
        self.signatures.get(request_id).map(|s| s.len() >= self.required).unwrap_or(false)
    }

    pub fn signature_count(&self, request_id: &str) -> usize {
        self.signatures.get(request_id).map(|s| s.len()).unwrap_or(0)
    }

    pub fn clear(&mut self, request_id: &str) {
        self.signatures.remove(request_id);
    }
}
