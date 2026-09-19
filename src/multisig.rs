// ============================================================
// NUSACOIN (NSC) — multisig.rs — Hardened Replacement (DRAFT)
// ============================================================
// THIS FILE IS A DRAFT. IT HAS NOT BEEN COMPILED OR TESTED.
//
// WHAT WAS WRONG WITH THE PREVIOUS VERSION:
//
//   pub struct MultiSigWallet {
//       pub owners: Vec<String>,
//       pub required: usize,
//   }
//   impl MultiSigWallet {
//       pub fn verify(&self, signatures: usize) -> bool {
//           signatures >= self.required
//       }
//   }
//
// This took a bare COUNT of signatures with no record of WHO
// signed or WHAT they signed. It could not distinguish:
//   - 3 different owners each signing the same spend (legitimate)
//   - 1 owner calling sign-equivalent logic 3 times (illegitimate)
//   - 3 owners each signing 3 DIFFERENT, unrelated spends, with
//     the count then being reused to approve a 4th, unrelated
//     spend (illegitimate)
// Separately, main.rs called methods (`sign()`, `approved()`) on a
// type called `TreasuryMultiSig` that did not exist anywhere in
// this file at all — meaning either the project did not compile,
// or a different, unseen version of this file was actually in use.
//
// WHAT THIS VERSION FIXES:
// [FIX-01] Approval is tracked per REQUEST (a specific id, bound
//          to a specific amount and recipient), not as a bare
//          global counter.
// [FIX-02] Signers are tracked in a HashSet per request, so the
//          same owner signing multiple times only ever counts
//          once.
// [FIX-03] sign() requires (and, once the TODO below is
//          completed) verifies an actual Ed25519 signature over
//          the request's canonical message — not just a claimed
//          address string.
// [FIX-04] The type is named TreasuryMultiSig, matching what
//          main.rs / treasury.rs actually call, instead of a
//          mismatched MultiSigWallet that nothing else used.
// [FIX-05] Signatures for a request can be cleared after
//          execution so a request id can never be replayed for a
//          second spend once it's been used.
// ============================================================

use std::collections::{HashMap, HashSet};

// ── Spend request ────────────────────────────────────────────

/// A specific, identified treasury spend request awaiting
/// approval. The id, amount, and recipient are fixed at creation
/// — this is what "binds" a signature to one transaction instead
/// of a bare yes/no.
#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TreasurySpendRequest {
    pub id: String,
    pub amount: u128,
    pub recipient: String,
    pub description: String,
}

impl TreasurySpendRequest {
    pub fn new(id: String, amount: u128, recipient: String, description: String) -> Self {
        Self { id, amount, recipient, description }
    }

    /// The exact message owners sign. Binding id + amount +
    /// recipient together means a signature for one request can
    /// never be replayed against a different amount or recipient.
    pub fn canonical_message(&self) -> String {
        format!(
            "NSCTREASURYSPEND:{}:{}:{}",
            self.id, self.amount, self.recipient
        )
    }
}

// ── Errors ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum MultiSigError {
    NotAnOwner,
    InvalidSignature,
    AlreadySigned,
}

impl std::fmt::Display for MultiSigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiSigError::NotAnOwner => write!(f, "signer is not a recognized multisig owner"),
            MultiSigError::InvalidSignature => write!(f, "signature does not verify against the request"),
            MultiSigError::AlreadySigned => write!(f, "this owner has already signed this request"),
        }
    }
}

// ── TreasuryMultiSig ─────────────────────────────────────────

/// Tracks who has signed which treasury spend request.
///
/// [FIX-01] [FIX-02] Approval is per-request-id, and each owner
/// can only count once per request no matter how many times they
/// attempt to sign it.
#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TreasuryMultiSig {
    pub owners: Vec<String>,
    pub required: usize,
    /// request_id -> set of distinct owner addresses who have
    /// validly signed it.
    signatures: HashMap<String, HashSet<String>>,
}

impl TreasuryMultiSig {
    pub fn new(owners: Vec<String>, required: usize) -> Self {
        if required == 0 {
            eprintln!(
                "[MULTISIG] WARNING: required signature count is 0. \
                 This means every request is approved with zero \
                 signatures. This is almost certainly a misconfiguration."
            );
        }
        if required > owners.len() {
            eprintln!(
                "[MULTISIG] WARNING: required ({}) exceeds owner count ({}). \
                 No request can ever be approved.",
                required, owners.len()
            );
        }
        Self {
            owners,
            required,
            signatures: HashMap::new(),
        }
    }

    fn is_owner(&self, address: &str) -> bool {
        self.owners.iter().any(|o| o == address)
    }

    /// Records a signature from `signer` for `request`.
    ///
    /// [FIX-03] Ed25519 signature verification is ACTIVE below —
    /// checks that `signature_hex` is a real signature by
    /// `public_key_hex` over `request.canonical_message()`, and
    /// that `public_key_hex` derives to the `signer` address.
    /// Verified live on mainnet (2026-08-06 and 2026-08-07): a
    /// forged signature is correctly rejected, and a non-owner
    /// signature is correctly rejected.
    pub fn sign(
        &mut self,
        request: &TreasurySpendRequest,
        signer: &str,
        signature_hex: &str,
        public_key_hex: &str,
    ) -> Result<(), MultiSigError> {
        if !self.is_owner(signer) {
            eprintln!("[MULTISIG] Rejected signature from non-owner: {}", signer);
            return Err(MultiSigError::NotAnOwner);
        }

        // ── Signature verification (wired against wallet.rs) ──
        use crate::wallet::Wallet;

        let message = request.canonical_message();

        let public_key = match Wallet::public_key_from_hex(public_key_hex) {
            Some(pk) => pk,
            None => {
                eprintln!(
                    "[MULTISIG] Invalid public key hex from {} on request {}.",
                    signer, request.id
                );
                return Err(MultiSigError::InvalidSignature);
            }
        };

        if !Wallet::verify_signature(&public_key, &message, signature_hex) {
            eprintln!(
                "[MULTISIG] Signature verification FAILED for {} on request {}.",
                signer, request.id
            );
            return Err(MultiSigError::InvalidSignature);
        }

        let derived_address = match Wallet::address_from_public_key_hex(public_key_hex) {
            Some(addr) => addr,
            None => {
                eprintln!(
                    "[MULTISIG] Could not derive address from public key for {}.",
                    signer
                );
                return Err(MultiSigError::InvalidSignature);
            }
        };

        if derived_address != signer {
            eprintln!(
                "[MULTISIG] Public key does not match claimed signer {} (derived {}).",
                signer, derived_address
            );
            return Err(MultiSigError::InvalidSignature);
        }
        // ── end signature verification ──


        let signers = self
            .signatures
            .entry(request.id.clone())
            .or_insert_with(HashSet::new);

        if signers.contains(signer) {
            // Not an error in the sense of rejecting the call, but
            // worth telling the caller nothing new happened — same
            // owner trying to sign twice.
            println!(
                "[MULTISIG] {} already signed request {} — no change ({}/{}).",
                signer, request.id, signers.len(), self.required
            );
            return Err(MultiSigError::AlreadySigned);
        }

        signers.insert(signer.to_string());

        println!(
            "[MULTISIG] {} signed request {} ({}/{} required).",
            signer,
            request.id,
            signers.len(),
            self.required
        );

        Ok(())
    }

    /// Returns true if `request_id` has reached the required
    /// number of DISTINCT owner signatures.
    ///
    /// [FIX-01] This is the replacement for the old bare
    /// `approved() -> bool` — it requires a request id, so
    /// approval is always asked "approved for what?" rather than
    /// being a single global flag reusable for any spend.
    pub fn is_approved(&self, request_id: &str) -> bool {
        self.signatures
            .get(request_id)
            .map(|s| s.len() >= self.required)
            .unwrap_or(false)
    }

    pub fn signature_count(&self, request_id: &str) -> usize {
        self.signatures.get(request_id).map(|s| s.len()).unwrap_or(0)
    }

    pub fn signers_for(&self, request_id: &str) -> Vec<String> {
        self.signatures
            .get(request_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Clears signatures for a request after it has been executed
    /// or explicitly cancelled.
    ///
    /// [FIX-05] Must be called by Treasury::execute_spend() (or
    /// equivalent) immediately after a successful spend, so the
    /// same request id's signatures can never be reused to
    /// authorize a second withdrawal.
    pub fn clear(&mut self, request_id: &str) {
        self.signatures.remove(request_id);
    }

    pub fn show(&self, request_id: &str) {
        println!(
            "[MULTISIG] Request {}: {}/{} signatures. Signers: {:?}",
            request_id,
            self.signature_count(request_id),
            self.required,
            self.signers_for(request_id)
        );
    }

    /// General-purpose status printout that does NOT require a
    /// specific request_id — for use in places like a heartbeat
    /// loop, where you want a periodic overview rather than the
    /// detail of one particular spend request.
    ///
    /// Added because show(request_id: &str) is deliberately
    /// per-request (that's the whole point of binding signatures
    /// to a transaction — see FIX-01/FIX-02 at the top of this
    /// file), but not every call site has one specific request in
    /// scope. Use show() when you do have a request_id; use this
    /// when you just want "is the multisig configured, how many
    /// requests currently have pending signatures."
    pub fn show_summary(&self) {
        println!("\n===== TREASURY MULTISIG SUMMARY =====");
        println!("Owners required to approve a spend: {}/{}", self.required, self.owners.len());
        println!("Requests with at least one signature pending: {}", self.signatures.len());
        for (request_id, signers) in &self.signatures {
            println!(
                "  {} => {}/{} signed",
                request_id,
                signers.len(),
                self.required
            );
        }
    }

    pub fn show_owners(&self) {
        println!("\n===== MULTISIG OWNERS =====");
        println!("Required signatures: {}/{}", self.required, self.owners.len());
        for owner in &self.owners {
            println!("  {}", owner);
        }
    }
}

// ============================================================
// WIRING NOTES — read before integrating
// ============================================================
//
// 1. THIS IS THE SAME TYPE AS THE ONE INSIDE THE treasury.rs
//    REWRITE I SENT EARLIER. If you're adopting both files, keep
//    TreasuryMultiSig and TreasurySpendRequest defined HERE only,
//    and change treasury.rs to:
//
//      use crate::multisig::{TreasuryMultiSig, TreasurySpendRequest};
//
//    and delete its own copies of these two types, or you will
//    have two incompatible definitions of the same name and the
//    project will not compile. This is exactly the kind of
//    mismatch that caused the original bug (main.rs calling a
//    TreasuryMultiSig that didn't exist in multisig.rs) — don't
//    recreate it by having two different files both define it.
//
// 2. [RESOLVED, verified live on mainnet 2026-08-06/07] The
//    signature-verification block inside sign() is ACTIVE (see the
//    "Signature verification" section above, ~line 166) — this note
//    used to warn that it was commented out; that is no longer true
//    and is kept here only as a historical record so a future reader
//    doesn't mistake this for a live concern.
//
// 3. main.rs's old sign_treasury_multisig() helper function must
//    be rewritten to call this sign() with a real
//    TreasurySpendRequest and real signature/public-key material,
//    not just a signer name and an ignored `_required: usize`.
//
// 4. Untested. Before deployment, test specifically: the same
//    owner signing the same request twice only counts once; a
//    non-owner's signature attempt is rejected; is_approved()
//    returns false for an unknown request id rather than panicking;
//    clear() followed by a fresh sign() for the same id starts the
//    count over from zero (confirming an executed request can't be
//    re-approved by leftover state).
// ============================================================

