// ============================================================
// NUSACOIN (NSC) — transaction.rs — Secure Mainnet Replacement
// ============================================================
// FIXES APPLIED:
// [FIX-01] Removed println! leaking message and signature
// [FIX-02] tx_hash excludes signature (signature is over msg)
// [FIX-03] verify() nonce cap raised to match chain.rs constant
// [FIX-04] verify() checks timestamp is not in the future
// [FIX-05] verify() checks fee > 0
// [FIX-06] verify_hash() is consistent with new() hash logic
// [FIX-07] message() format locked with prefix to prevent
//           cross-context signature reuse
// [FIX-08] new() uses saturating arithmetic, no unwrap()
// [FIX-09] age_seconds() guards against clock skew underflow
// [FIX-10] All unwrap() replaced with safe alternatives
// ============================================================

use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use crate::hash::calculate_hash;

// ── Constants ─────────────────────────────────────────────────

/// Maximum allowed nonce value — must match chain.rs.
const MAX_NONCE: u64 = 1_000_000_000;

/// Maximum seconds a transaction may be ahead of wall time.
const MAX_FUTURE_DRIFT_SECS: u64 = 7_200;

/// Maximum age of a transaction before it is considered stale.
const MAX_TX_AGE_SECS: u64 = 86_400; // 24 hours

// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub sender:     String,
    pub receiver:   String,
    pub amount:     u128,
    pub fee:        u128,
    pub nonce:      u64,
    pub timestamp:  u64,
    pub signature:  String,
    /// [FIX-02] tx_hash is over canonical fields only,
    /// NOT over the signature itself.
    pub tx_hash:    String,
    pub public_key: String,
    /// Signature scheme: 0 = Ed25519 (legacy NSC... addresses),
    /// 1 = secp256k1/EVM (0x... addresses via MetaMask etc).
    /// serde(default) keeps old persisted data (pre-dating this
    /// field) loading correctly as scheme 0.
    #[serde(default)]
    pub scheme: u8,
    /// Raw RLP-encoded EVM tx bytes (hex, no 0x prefix), present
    /// only when scheme == 1. Needed to re-verify the secp256k1
    /// signature at mempool re-validation / mining time.
    #[serde(default)]
    pub raw_evm_tx: Option<String>,
    /// Original Ethereum-style keccak256 tx hash (0x...), present
    /// only when scheme == 1. Used for eth_getTransactionReceipt
    /// lookups. Distinct from tx_hash (this chain's internal hash).
    #[serde(default)]
    pub eth_tx_hash: Option<String>,
}

impl Transaction {

    // ── Constructor ───────────────────────────────────────────

    /// Creates a new Transaction and computes its hash.
    ///
    /// [FIX-08] No unwrap() — falls back to 0 on clock error.
    /// [FIX-02] tx_hash computed over canonical fields only.
    pub fn new(
        sender:     String,
        receiver:   String,
        amount:     u128,
        fee:        u128,
        nonce:      u64,
        public_key: String,
        signature:  String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // [FIX-02] Hash over canonical fields — NOT signature.
        // This makes the hash deterministic and verifiable.
        let tx_hash = Self::compute_hash(
            &sender,
            &receiver,
            &public_key,
            amount,
            fee,
            nonce,
            timestamp,
        );

        Self {
            sender,
            receiver,
            amount,
            fee,
            nonce,
            public_key,
            timestamp,
            signature,
            tx_hash,
            scheme: 0,
            raw_evm_tx: None,
            eth_tx_hash: None,
        }
    }

    /// Creates a new EVM-originated Transaction (scheme = 1).
    ///
    /// Unlike new(), there is no persistent public_key field —
    /// ownership is proven by secp256k1 signature recovery over
    /// raw_evm_tx (see evm_recover()/verify_signature()).
    pub fn new_evm(
        sender:      String,
        receiver:    String,
        amount:      u128,
        fee:         u128,
        nonce:       u64,
        raw_evm_tx:  String,
        eth_tx_hash: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let tx_hash = Self::compute_hash_evm(
            &sender,
            &receiver,
            &raw_evm_tx,
            amount,
            fee,
            nonce,
            timestamp,
        );

        Self {
            sender,
            receiver,
            amount,
            fee,
            nonce,
            timestamp,
            signature: String::new(),
            tx_hash,
            public_key: String::new(),
            scheme: 1,
            raw_evm_tx: Some(raw_evm_tx),
            eth_tx_hash: Some(eth_tx_hash),
        }
    }

    // ── Hash computation ──────────────────────────────────────

    /// Computes the canonical transaction hash.
    ///
    /// [FIX-02] Signature is excluded from the hash so the
    /// hash is stable and can be computed by any node.
    /// [FIX-06] Must match verify_hash() exactly.
    fn compute_hash(
        sender:     &str,
        receiver:   &str,
        public_key: &str,
        amount:     u128,
        fee:        u128,
        nonce:      u64,
        timestamp:  u64,
    ) -> String {
        let content = format!(
            "NSCTX:{}:{}:{}:{}:{}:{}:{}",
            sender,
            receiver,
            public_key,
            amount,
            fee,
            nonce,
            timestamp,
        );
        calculate_hash(&content)
    }

    /// Computes the canonical hash for an EVM-originated tx.
    /// Binds the hash to the exact raw signed payload, mirroring
    /// how compute_hash() binds to public_key for Ed25519 txs.
    fn compute_hash_evm(
        sender:     &str,
        receiver:   &str,
        raw_evm_tx: &str,
        amount:     u128,
        fee:        u128,
        nonce:      u64,
        timestamp:  u64,
    ) -> String {
        let content = format!(
            "NSCTX-EVM:{}:{}:{}:{}:{}:{}:{}",
            sender,
            receiver,
            raw_evm_tx,
            amount,
            fee,
            nonce,
            timestamp,
        );
        calculate_hash(&content)
    }

    // ── Canonical message for signing ─────────────────────────

    /// Returns the canonical message that is signed by the
    /// sender wallet and verified during transaction validation.
    ///
    /// [FIX-07] Prefixed with "NSCMSG:" to prevent this
    /// signature from being reused for other message types
    /// (e.g. vote signatures in consensus.rs).
    ///
    /// MUST match wallet.rs sign_transaction() exactly.
    pub fn message(
        sender:   &str,
        receiver: &str,
        amount:   u128,
        fee:      u128,
        nonce:    u64,
    ) -> String {
        format!(
            "NSCMSG:{}:{}:{}:{}:{}",
            sender,
            receiver,
            amount,
            fee,
            nonce,
        )
    }

    // ── Verification ──────────────────────────────────────────

    /// Re-derives the sender address from raw_evm_tx via
    /// secp256k1 signature recovery. Returns None if this is
    /// not an EVM tx or recovery fails for any reason.
    fn evm_recover(&self) -> Option<String> {
        let raw_hex = self.raw_evm_tx.as_deref()?;
        let raw_hex_trimmed = raw_hex.trim_start_matches("0x");
        let raw_bytes = hex::decode(raw_hex_trimmed).ok()?;
        let decoded = crate::evm_tx::decode_legacy_tx(&raw_bytes).ok()?;
        crate::evm_tx::recover_sender(&decoded, crate::evm_tx::NSC_EVM_CHAIN_ID).ok()
    }

    /// Verifies the signature on this transaction. For scheme 0
    /// (Ed25519/NSC...) verifies against the stored public key.
    /// For scheme 1 (EVM/0x...) re-derives the sender via
    /// secp256k1 recovery over raw_evm_tx and compares addresses.
    ///
    /// [FIX-01] Removed println! leaking message and signature.
    pub fn verify_signature(&self) -> bool {
        if self.scheme == 1 {
            return match self.evm_recover() {
                Some(recovered) => recovered.to_lowercase() == self.sender.to_lowercase(),
                None => false,
            };
        }

        let public_key = match crate::wallet::Wallet
            ::public_key_from_hex(&self.public_key)
        {
            Some(pk) => pk,
            None     => return false,
        };

        let message = Self::message(
            &self.sender,
            &self.receiver,
            self.amount,
            self.fee,
            self.nonce,
        );

        // [FIX-01] No println of message or signature.
        crate::wallet::Wallet::verify_signature(
            &public_key,
            &message,
            &self.signature,
        )
    }

    /// Verifies that the sender address matches the signer.
    /// Scheme 0: matches stored Ed25519 public key -> address.
    /// Scheme 1: matches secp256k1-recovered address (ownership
    /// is self-certifying via signature recovery for EVM txs).
    pub fn verify_sender_ownership(&self) -> bool {
        if self.scheme == 1 {
            return match self.evm_recover() {
                Some(recovered) => recovered.to_lowercase() == self.sender.to_lowercase(),
                None => false,
            };
        }

        let address = match crate::wallet::Wallet
            ::address_from_public_key_hex(&self.public_key)
        {
            Some(addr) => addr,
            None       => return false,
        };

        address == self.sender
    }

    /// Verifies the transaction hash has not been tampered with.
    ///
    /// [FIX-06] Uses same compute_hash() as new() — consistent.
    pub fn verify_hash(&self) -> bool {
        let expected = if self.scheme == 1 {
            let raw = self.raw_evm_tx.as_deref().unwrap_or("");
            Self::compute_hash_evm(
                &self.sender,
                &self.receiver,
                raw,
                self.amount,
                self.fee,
                self.nonce,
                self.timestamp,
            )
        } else {
            Self::compute_hash(
                &self.sender,
                &self.receiver,
                &self.public_key,
                self.amount,
                self.fee,
                self.nonce,
                self.timestamp,
            )
        };
        expected == self.tx_hash
    }

    /// Full transaction validity check.
    ///
    /// [FIX-03] Nonce cap matches chain.rs MAX_NONCE.
    /// [FIX-04] Rejects transactions timestamped in the future.
    /// [FIX-05] Fee must be > 0.
    pub fn verify(&self) -> bool {
        if self.sender.is_empty() {
            return false;
        }
        if self.receiver.is_empty() {
            return false;
        }
        if self.sender == self.receiver {
            return false;
        }
        if self.amount == 0 {
            return false;
        }
        // [FIX-05] Fee must be non-zero.
        if self.fee == 0 {
            return false;
        }
        if self.scheme == 0 {
            if self.signature.is_empty() {
                return false;
            }
            if self.public_key.is_empty() {
                return false;
            }
        } else if self.scheme == 1 {
            if self.raw_evm_tx.is_none() {
                return false;
            }
        } else {
            return false;
        }
        // [FIX-03] Nonce within bounds.
        if self.nonce >= MAX_NONCE {
            return false;
        }
        if self.timestamp == 0 {
            return false;
        }
        // [FIX-04] Timestamp not in the future.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if self.timestamp > now + MAX_FUTURE_DRIFT_SECS {
            return false;
        }
        if !self.verify_hash() {
            return false;
        }
        if !self.verify_signature() {
            return false;
        }
        if !self.verify_sender_ownership() {
            return false;
        }
        true
    }

    // ── Age helper ────────────────────────────────────────────

    /// Returns the age of the transaction in seconds.
    ///
    /// [FIX-09] Guards against underflow when clock skew
    /// makes timestamp appear to be in the future.
    pub fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.timestamp)
    }

    /// Returns true if the transaction is still within the
    /// maximum allowed mempool age.
    pub fn is_fresh(&self) -> bool {
        self.age_seconds() <= MAX_TX_AGE_SECS
    }
}
// ============================================================
// END OF transaction.rs
// ============================================================

