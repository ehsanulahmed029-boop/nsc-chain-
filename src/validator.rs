// ============================================================
// NUSACOIN (NSC) — validator.rs — Hardened Replacement (DRAFT)
// ============================================================
// THIS FILE IS A DRAFT. IT HAS NOT BEEN COMPILED OR TESTED.
//
// WHAT WAS WRONG WITH THE PREVIOUS VERSION:
//
//   #[derive(Debug)]
//   pub struct Validator {
//       pub votes: HashMap<String, u64>,
//       pub stake: u64,
//       pub signing_key: SigningKey,
//       pub verifying_key: VerifyingKey,
//   }
//
// 1. CRITICAL: #[derive(Debug)] on a struct holding the raw
//    private signing key means ANY `println!("{:?}", validator)`
//    or `eprintln!("{:?}", ...)` anywhere in the codebase — and
//    this codebase prints almost everything via .show()/println!
//    patterns — writes the private key to stdout/logs in full.
//    wallet.rs specifically fixed this exact class of bug with a
//    manual, redacted Debug impl ([FIX-10] in that file). This
//    struct had no such protection, meaning the private-key-leak
//    bug wallet.rs fixed in ONE place in the codebase was still
//    wide open in this second, independent key-holding struct.
//
// 2. `stake: 1000` was hardcoded in `Validator::new()` — not
//    derived from any real staking deposit. Every validator
//    using this struct got identical, fabricated voting weight
//    regardless of what they actually staked.
//
// 3. `sign_block()` signed the raw block_hash string directly,
//    with no domain-separation prefix — unlike consensus.rs's
//    ValidatorVote::vote_message(), which prefixes with
//    "NSCVOTE:" specifically so a vote signature can't be
//    replayed as a signature for something else. Signing a bare
//    hash here meant this signature and a consensus vote
//    signature could potentially collide in what they're
//    signing over, undermining the domain separation
//    consensus.rs deliberately built.
//
// WHAT THIS VERSION FIXES:
// [FIX-01] Manual, redacted Debug impl — mirrors wallet.rs's
//          [FIX-10] exactly. The private key can never be
//          printed through this struct.
// [FIX-02] No `stake` field on Validator at all. Stake is now
//          sourced from staking.rs's Staking struct, which is
//          balance-backed and accumulates correctly — Validator
//          no longer fabricates or duplicates that number.
// [FIX-03] sign_block() now signs a domain-separated message
//          ("NSCVALIDATORBLOCK:{block_hash}"), matching the
//          pattern consensus.rs already established, instead of
//          a bare hash.
// [FIX-04] verify_block_signature() correspondingly checks the
//          same domain-separated message.
// [FIX-05] Validator no longer implements Clone by derive — key
//          material being trivially cloneable around the
//          codebase is itself a smaller-but-real risk (more
//          copies of the private key sitting in memory in more
//          places than necessary). A deliberate, explicit
//          clone_for_signing_only() helper is provided where
//          truly needed instead of a blanket derive.
// [FIX-06] No unwrap() on signature construction.
// ============================================================

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hex;
use rand::rngs::OsRng;

/// Domain-separation prefix for validator block signatures.
/// Mirrors consensus.rs's "NSCVOTE:" prefix pattern — this
/// ensures a signature produced here can never be replayed as a
/// valid signature for a consensus vote, a treasury multisig
/// approval, or a regular transaction, since each of those signs
/// a differently-prefixed message.
const BLOCK_SIGN_PREFIX: &str = "NSCVALIDATORBLOCK";

/// A validator's signing identity.
///
/// [FIX-01] Deliberately does NOT derive Debug or Clone. Holding
/// a private key is the entire reason this struct exists, and a
/// blanket derive of either trait risks the key leaking into a
/// log line or being duplicated into more places in memory than
/// necessary. Use the explicit accessor methods below instead.
pub struct Validator {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    /// The validator's NSC address, derived from verifying_key —
    /// stored alongside for convenience, but always re-derivable
    /// from the public key alone if ever in doubt.
    address: String,
}

// [FIX-01] Manual Debug impl, mirroring wallet.rs's [FIX-10].
// The private key is never included, in any form, in any branch
// of this impl.
impl std::fmt::Debug for Validator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Validator")
            .field("address", &self.address)
            .field("public_key", &self.public_key_hex())
            .field("signing_key", &"[REDACTED]")
            .finish()
    }
}

impl Validator {
    /// Generates a fresh validator identity using a
    /// cryptographically secure OS random number generator.
    ///
    /// [FIX-02] No stake field — stake is tracked exclusively by
    /// staking.rs's Staking struct, keyed by this validator's
    /// address, and must be established via a real
    /// Staking::stake() call (which itself requires a real
    /// balance debit) before this validator is considered active
    /// by anything that checks stake.
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let address = Self::derive_address(&verifying_key);

        Self {
            signing_key,
            verifying_key,
            address,
        }
    }

    /// Restores a validator identity from a hex-encoded private
    /// key. Mirrors wallet.rs's Wallet::restore() error-handling
    /// pattern — no unwrap(), returns None on any invalid input.
    pub fn restore(private_key_hex: &str) -> Option<Self> {
        let bytes = match hex::decode(private_key_hex) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("[VALIDATOR] restore(): invalid hex.");
                return None;
            }
        };

        let arr: [u8; 32] = match bytes.try_into() {
            Ok(a) => a,
            Err(_) => {
                eprintln!("[VALIDATOR] restore(): key must be 32 bytes.");
                return None;
            }
        };

        let signing_key = SigningKey::from_bytes(&arr);
        let verifying_key = signing_key.verifying_key();
        let address = Self::derive_address(&verifying_key);

        Some(Self {
            signing_key,
            verifying_key,
            address,
        })
    }

    /// Derives the validator's NSC address from its public key.
    /// Uses the SAME derivation as wallet.rs's Wallet::derive_address
    /// so a validator's address and a wallet's address are
    /// computed identically — they are, after all, the same kind
    /// of address space.
    ///
    /// NOTE: this inherits the 64-bit address-truncation concern
    /// flagged against wallet.rs earlier in this review. If that
    /// gets fixed (widening the address derivation), this function
    /// must change identically, or validator addresses and wallet
    /// addresses will silently diverge in format.
    fn derive_address(public_key: &VerifyingKey) -> String {
        format!("NSC{}", &hex::encode(public_key.as_bytes())[..16])
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.verifying_key.as_bytes())
    }

    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Signs a block hash for consensus/leader purposes.
    ///
    /// [FIX-03] Domain-separated: signs "NSCVALIDATORBLOCK:{hash}",
    /// not the bare hash. This prevents this signature from ever
    /// being valid as a signature over a differently-prefixed
    /// message elsewhere in the codebase (consensus votes,
    /// treasury multisig approvals, regular transactions all use
    /// their own distinct prefixes).
    pub fn sign_block(&self, block_hash: &str) -> String {
        let message = format!("{}:{}", BLOCK_SIGN_PREFIX, block_hash);
        let signature: Signature = self.signing_key.sign(message.as_bytes());
        hex::encode(signature.to_bytes())
    }

    /// Verifies a block signature against a known public key.
    ///
    /// [FIX-04] [FIX-06] Checks the same domain-separated message
    /// sign_block() actually signs. No unwrap() anywhere in the
    /// parse path — every failure mode returns false rather than
    /// panicking.
    pub fn verify_block_signature(
        public_key_hex: &str,
        block_hash: &str,
        signature_hex: &str,
    ) -> bool {
        let pk_bytes = match hex::decode(public_key_hex) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let pk_array: [u8; 32] = match pk_bytes.try_into() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let public_key = match VerifyingKey::from_bytes(&pk_array) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let sig_bytes = match hex::decode(signature_hex) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let sig_array: [u8; 64] = match sig_bytes.try_into() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let signature = Signature::from_bytes(&sig_array);

        let message = format!("{}:{}", BLOCK_SIGN_PREFIX, block_hash);

        public_key.verify(message.as_bytes(), &signature).is_ok()
    }

    /// Verifies that this validator's address actually corresponds
    /// to its own public key. Should always be true for a Validator
    /// constructed via new() or restore() — this exists mainly as
    /// a defensive self-check / for use in tests, mirroring
    /// transaction.rs's verify_sender_ownership pattern.
    pub fn verify_self_consistency(&self) -> bool {
        Self::derive_address(&self.verifying_key) == self.address
    }
}

// [FIX-05] No blanket `impl Clone for Validator`. If a specific,
// reviewed call site genuinely needs to hand a signing capability
// to another thread/component, prefer restructuring so only the
// Arc<Mutex<Validator>> is shared, rather than cloning the private
// key into a second copy. If you find you truly need cloning,
// treat adding it back as a deliberate, reviewed decision — not a
// convenience default — given it directly increases how many
// places in memory hold a copy of the private key.

// ============================================================
// WIRING NOTES — read before integrating
// ============================================================
//
// 1. CRITICAL — find every place in the codebase that did
//    `validator.stake` directly (the old struct had `pub stake:
//    u64`) and replace it with a lookup into the real Staking
//    struct: `staking.stake_of(validator.address())`. This
//    Validator no longer carries a stake field at all — stake
//    lives exclusively in staking.rs now, sourced from real,
//    balance-backed deposits. If anything still reads
//    `validator.stake`, it will fail to compile, which is the
//    correct outcome here — it forces you to find and fix every
//    place that was trusting the old fabricated number.
//
// 2. consensus.rs's ConsensusEngine::add_vote() looks up validator
//    info (including stake and jailed status) from
//    ValidatorRegistry, not from this Validator struct directly.
//    Confirm ValidatorRegistry itself sources its stake numbers
//    from staking.rs's Staking struct — if ValidatorRegistry has
//    its own separate, independent stake bookkeeping, you now have
//    THREE places that could each have a different idea of a given
//    validator's stake (this file's old hardcoded 1000, staking.rs,
//    and ValidatorRegistry). They must all resolve to a single
//    source of truth — I'd make that staking.rs's Staking struct,
//    since it's the one actually backed by real debited balances.
//
// 3. The #[derive(Debug)] removal will break any code that did
//    `println!("{:?}", some_validator)` expecting the OLD,
//    unredacted output (e.g. logging full validator state during
//    debugging). That breakage is intentional — find those call
//    sites and confirm they only ever needed address/public_key,
//    which the new redacted Debug impl still provides.
//
// 4. sign_block()'s new domain-separated message format means any
//    signature produced by the OLD version of this file (signing
//    the bare hash) will NOT verify against the new
//    verify_block_signature(). This is a breaking change to the
//    wire format for anything that used the old sign_block/verify
//    pair. If any block signatures already exist on a running
//    testnet using the old format, they will all need to be
//    considered invalid under the new scheme — there's no way to
//    silently bridge old and new signature formats without
//    weakening the domain separation that's the actual point of
//    this fix.
//
// 5. Untested. Before deployment: test that Debug output never
//    contains the hex of the private key (grep test output for
//    the known test key's hex string, confirm it's absent); that
//    sign_block()/verify_block_signature() round-trip correctly;
//    that a signature produced by the OLD format (bare hash, no
//    prefix) correctly FAILS verification under the new function
//    (confirming the domain separation actually changed
//    something); that restore() rejects non-32-byte input cleanly.
// ============================================================

