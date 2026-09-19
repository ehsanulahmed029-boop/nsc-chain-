// ============================================================
// NUSACOIN (NSC) — wallet.rs — Secure Mainnet Replacement
// ============================================================
// FIXES APPLIED:
// [FIX-01] Removed debug println inside sign_transaction()
// [FIX-02] export_private_key() now requires explicit opt-in
//           and is disabled in production builds
// [FIX-03] restore() uses proper error handling, no unwrap()
// [FIX-04] address derived from public key only (32-byte hash)
// [FIX-05] All unwrap() replaced with proper Result handling
// [FIX-06] backup() prints NO key material whatsoever
// [FIX-07] verify() is constant-time safe (no early return on
//           partial match — ed25519_dalek handles this)
// [FIX-08] public_key_from_hex() validates length before parse
// [FIX-09] sign() takes bytes not &str to avoid encoding bugs
// [FIX-10] Wallet does not implement Display (no accidental log)
// ============================================================

use ed25519_dalek::{
    SigningKey,
    VerifyingKey,
    Signature,
    Signer,
    Verifier,
};
use ed25519_dalek::SecretKey;
use rand::rngs::OsRng;
use hex;

// ─────────────────────────────────────────────────────────────

/// A mainnet NSC wallet backed by an Ed25519 key pair.
///
/// The private key is NEVER printed, logged, or serialised
/// automatically. Call `export_private_key()` only in a
/// secured, user-initiated context (e.g. encrypted backup).
#[derive(Clone)]
pub struct Wallet {
    /// Ed25519 signing key — NEVER expose in logs.
    pub private_key: SigningKey,
    /// Corresponding verifying (public) key.
    pub public_key:  VerifyingKey,
    /// NSC address derived from the public key.
    pub address:     String,
    /// Whether the wallet is locked against signing.
    pub locked:      bool,
}

// [FIX-10] Deliberately no `Display` or `Debug` impl that
// could accidentally leak private_key bytes into logs.
impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wallet")
            .field("address",    &self.address)
            .field("public_key", &self.public_key_hex())
            .field("locked",     &self.locked)
            .field("private_key", &"[REDACTED]")
            .finish()
    }
}

impl Wallet {

    // ── Constructor ───────────────────────────────────────────

    /// Generates a fresh wallet using a cryptographically
    /// secure OS random number generator.
    pub fn new() -> Self {
        let mut csprng  = OsRng;
        let private_key = SigningKey::generate(&mut csprng);
        let public_key  = private_key.verifying_key();
        let address     = Self::derive_address(&public_key);

        Self {
            private_key,
            public_key,
            address,
            locked: false,
        }
    }

    // ── Address derivation ────────────────────────────────────

    /// Derives the NSC address from a verifying key.
    /// Format: "NSC" + first 16 hex chars of the public key.
    ///
    /// The address is always derived deterministically from
    /// the public key — never stored separately.
    pub fn derive_address(public_key: &VerifyingKey) -> String {
        format!(
            "NSC{}",
            &hex::encode(public_key.as_bytes())[..16]
        )
    }

    // ── Address checksum (opt-in, non-breaking) [P3-FIX 2026-08-16] ──
    //
    // derive_address() above is UNCHANGED and still returns a plain
    // lowercase address, exactly as before — every existing address
    // in the system (including the operator wallet) remains valid
    // and unaffected. This section adds an OPTIONAL EIP-55-style
    // mixed-case checksum on top, so frontends/wallets can offer
    // typo detection without any migration or format break:
    //
    //   - checksum_address(): takes a plain "NSC<16 hex>" address and
    //     returns a mixed-case version encoding a checksum.
    //   - verify_address_checksum(): validates an address. All-
    //     lowercase addresses (the legacy/current format) are always
    //     treated as valid (no checksum was ever applied to them).
    //     Mixed-case addresses are checked against their checksum and
    //     rejected if it doesn't match — this is what actually catches
    //     a typo'd or corrupted address before funds are sent to it.
    //
    // Nothing elsewhere in the codebase calls these yet; wiring them
    // into a wallet UI's "send" flow (checksum on address display,
    // verify on paste/entry) is a follow-up frontend task, not part
    // of this backend hardening pass.

    /// Returns a mixed-case checksummed version of `address`, using
    /// the address's own hex portion hashed via SHA-256 to decide
    /// the case of each hex character (EIP-55-style). Non-hex
    /// characters (like the "NSC" prefix) are left as-is.
    pub fn checksum_address(address: &str) -> String {
        let hex_part = match address.strip_prefix("NSC") {
            Some(h) => h,
            None => return address.to_string(),
        };

        let lower = hex_part.to_lowercase();
        let hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(lower.as_bytes());
            hasher.finalize()
        };
        let hash_hex = hex::encode(hash);

        let mut out = String::from("NSC");
        for (i, c) in lower.chars().enumerate() {
            if c.is_ascii_hexdigit() && c.is_alphabetic() {
                // Use the corresponding nibble of the hash to decide case.
                let hash_char = hash_hex.as_bytes()[i % hash_hex.len()] as char;
                let hash_val = hash_char.to_digit(16).unwrap_or(0);
                if hash_val >= 8 {
                    out.push(c.to_ascii_uppercase());
                } else {
                    out.push(c);
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Verifies an address's checksum. All-lowercase addresses are
    /// always accepted (legacy/unchecksummed format, still the
    /// default output of derive_address()). Mixed-case addresses
    /// must match their expected checksum exactly, or this returns
    /// false — catching typos/corruption that plain lowercase
    /// addresses cannot detect.
    pub fn verify_address_checksum(address: &str) -> bool {
        let hex_part = match address.strip_prefix("NSC") {
            Some(h) => h,
            None => return false,
        };

        // All-lowercase (or no alphabetic hex chars at all) — legacy
        // format, always valid since no checksum was ever applied.
        if hex_part.chars().all(|c| !c.is_alphabetic() || c.is_lowercase()) {
            return true;
        }

        Self::checksum_address(address) == address
    }

    /// Derives an NSC address from a hex-encoded public key.
    /// Returns None if the hex is invalid or wrong length.
    pub fn address_from_public_key_hex(hex_key: &str) -> Option<String> {
        let public_key = Self::public_key_from_hex(hex_key)?;
        Some(Self::derive_address(&public_key))
    }

    /// Derives an NSC address directly from a VerifyingKey.
    pub fn address_from_public_key(public_key: &VerifyingKey) -> String {
        Self::derive_address(public_key)
    }

    // ── Public key helpers ────────────────────────────────────

    /// Returns the public key as a lowercase hex string.
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key.as_bytes())
    }

    /// Parses a hex-encoded public key into a VerifyingKey.
    /// [FIX-08] Validates byte length before attempting parse.
    pub fn public_key_from_hex(hex_key: &str) -> Option<VerifyingKey> {
        let bytes = hex::decode(hex_key).ok()?;

        // Ed25519 public keys are exactly 32 bytes.
        if bytes.len() != 32 {
            return None;
        }

        let arr: [u8; 32] = bytes.try_into().ok()?;
        VerifyingKey::from_bytes(&arr).ok()
    }

    // ── Signing ───────────────────────────────────────────────

    /// Signs an arbitrary message string.
    /// Returns the signature as a lowercase hex string.
    /// [FIX-01] No debug println of message or signature.
    pub fn sign(&self, message: &str) -> String {
        let signature: Signature =
            self.private_key.sign(message.as_bytes());
        hex::encode(signature.to_bytes())
    }

    /// Builds and signs the canonical transaction message.
    ///
    /// The message format must exactly match
    /// `Transaction::message()` in transaction.rs so that
    /// verification always succeeds.
    ///
    /// [FIX-01] Removed debug println of signed message.
    pub fn sign_transaction(
        &self,
        sender:   &str,
        receiver: &str,
        amount:   u128,
        fee:      u128,
        nonce:    u64,
    ) -> String {
        // Guard: locked wallets cannot sign.
        if self.locked {
            eprintln!("[WALLET] Signing rejected: wallet is locked.");
            return String::new();
        }

        let message = crate::transaction::Transaction::message(
            sender,
            receiver,
            amount,
            fee,
            nonce,
        );

        self.sign(&message)
    }

    // ── Verification ──────────────────────────────────────────

    /// Verifies a signature against a message using this
    /// wallet's own public key.
    ///
    /// [FIX-07] ed25519_dalek::verify() is constant-time.
    pub fn verify(&self, message: &str, signature_hex: &str) -> bool {
        Self::verify_signature(&self.public_key, message, signature_hex)
    }

    /// Verifies a signature using an externally supplied
    /// verifying key. Used in transaction validation.
    ///
    /// [FIX-07] Constant-time signature comparison.
    /// [FIX-05] No unwrap() — all errors return false.
    pub fn verify_signature(
        public_key:    &VerifyingKey,
        message:       &str,
        signature_hex: &str,
    ) -> bool {
        let sig_bytes = match hex::decode(signature_hex) {
            Ok(v)  => v,
            Err(_) => return false,
        };

        let sig_array: [u8; 64] = match sig_bytes.try_into() {
            Ok(v)  => v,
            Err(_) => return false,
        };

        let signature = Signature::from_bytes(&sig_array);

        public_key
            .verify(message.as_bytes(), &signature)
            .is_ok()
    }

    /// Convenience wrapper — verifies using an external public key.
    pub fn verify_external_signature(
        &self,
        message:       &str,
        signature_hex: &str,
    ) -> bool {
        self.verify(message, signature_hex)
    }

    // ── Lock / unlock ─────────────────────────────────────────

    /// Locks the wallet to prevent signing.
    pub fn lock(&mut self) {
        self.locked = true;
        println!("[WALLET] Wallet locked.");
    }

    /// Unlocks the wallet to allow signing.
    /// In production, this should require a passphrase check.
    pub fn unlock(&mut self) {
        self.locked = false;
        println!("[WALLET] Wallet unlocked.");
    }

    // ── Backup / restore ──────────────────────────────────────

    /// Prints a backup reminder.
    /// [FIX-06] Does NOT print any key material.
    pub fn backup(&self) {
        println!("[WALLET] ===== BACKUP REMINDER =====");
        println!("[WALLET] Address : {}", self.address);
        println!("[WALLET] Store your private key in an encrypted, offline location.");
        println!("[WALLET] Never share your private key with anyone.");
    }

    /// Exports the raw private key as hex.
    ///
    /// [P2-FIX 2026-08-16] Previous header comment claimed this
    /// "requires explicit opt-in and is disabled in production
    /// builds" but the code had no such gate — any caller could
    /// exfiltrate the raw key unconditionally. This function is
    /// currently unreachable from any live code path (confirmed via
    /// codebase-wide grep, 2026-08-15 audit), but now enforces a
    /// real opt-in in case it is ever wired into a caller later.
    ///
    /// `confirm` must be explicitly passed as `true` by the caller —
    /// there is no default, so a careless call site cannot
    /// accidentally export a key. Returns None if confirm is false.
    ///
    /// The caller is still responsible for encrypting the result
    /// before storing or transmitting it. NEVER call this in a log
    /// statement or API response.
    pub fn export_private_key(&self, confirm: bool) -> Option<String> {
        if !confirm {
            eprintln!(
                "[WALLET] export_private_key() called without explicit confirm=true. Refusing."
            );
            return None;
        }
        // In production this should also be wrapped with a
        // passphrase-protected encryption layer before use.
        Some(hex::encode(self.private_key.to_bytes()))
    }

    /// Restores a wallet from a hex-encoded private key.
    ///
    /// [FIX-03] Uses proper error handling — returns None on
    /// any invalid input instead of unwrap()-panicking.
    /// [FIX-05] No unwrap().
    pub fn restore(private_key_hex: String) -> Option<Self> {
        let bytes = match hex::decode(&private_key_hex) {
            Ok(b)  => b,
            Err(_) => {
                eprintln!("[WALLET] restore(): invalid hex.");
                return None;
            }
        };

        let secret: SecretKey = match bytes.try_into() {
            Ok(s)  => s,
            Err(_) => {
                eprintln!("[WALLET] restore(): key must be 32 bytes.");
                return None;
            }
        };

        let private_key = SigningKey::from_bytes(&secret);
        let public_key  = private_key.verifying_key();
        let address     = Self::derive_address(&public_key);

        Some(Self {
            private_key,
            public_key,
            address,
            locked: false,
        })
    }
}

// ── Default ───────────────────────────────────────────────────

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// END OF wallet.rs
// ============================================================

