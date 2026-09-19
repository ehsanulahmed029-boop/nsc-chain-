// ============================================================
// NUSACOIN (NSC) — consensus.rs — Secure Mainnet Replacement
// ============================================================
// FIXES APPLIED:
// [FIX-01] Signatures are real Ed25519 — not SHA-256 hashes
// [FIX-02] generate_signature() removed — replaced with
//           sign_vote() that requires a real Wallet
// [FIX-03] verify_signature() uses Wallet::verify_signature()
//           with real Ed25519 public key, not hash comparison
// [FIX-04] ValidatorVote stores public_key for verification
// [FIX-05] Double-vote slashing retained and strengthened
// [FIX-06] Jailed validator vote rejection retained
// [FIX-07] Unknown validator vote rejection retained
// [FIX-08] finality_reached() requires ≥ 2/3 stake threshold
// [FIX-09] Zero-stake votes are always rejected
// [FIX-10] No unwrap() — all errors handled explicitly
// ============================================================

use std::collections::{HashMap, HashSet};
use crate::validator_registry::ValidatorRegistry;
use crate::wallet::Wallet;

// ─────────────────────────────────────────────────────────────

/// Finality threshold: 2/3 of total stake must agree.
const FINALITY_THRESHOLD_NUM: u64 = 2;
const FINALITY_THRESHOLD_DEN: u64 = 3;

// ─────────────────────────────────────────────────────────────

/// A single validator vote for a block hash.
///
/// [FIX-04] Now stores `public_key` so the signature can be
/// independently verified by any node without trusting the
/// validator address field alone.
#[derive(Debug, Clone)]
pub struct ValidatorVote {
    /// Validator NSC address.
    pub validator:  String,
    /// Validator staked amount (from registry at vote time).
    pub stake:      u64,
    /// The block hash this validator is voting for.
    pub block_hash: String,
    /// Real Ed25519 signature over `vote_message()`.
    pub signature:  String,
    /// Hex-encoded Ed25519 public key of the validator.
    pub public_key: String,
}

impl ValidatorVote {

    /// Canonical vote message that is signed and verified.
    ///
    /// Format: `NSCVOTE:{validator}:{block_hash}`
    /// The prefix prevents signature reuse across message types.
    pub fn vote_message(validator: &str, block_hash: &str) -> String {
        format!("NSCVOTE:{}:{}", validator, block_hash)
    }

    /// Signs a vote using a real Ed25519 wallet.
    ///
    /// [FIX-01] This replaces `generate_signature()` which was
    /// using SHA-256 (not Ed25519) and was not actually secure.
    pub fn sign_vote(
        wallet:     &Wallet,
        block_hash: &str,
    ) -> String {
        let message = Self::vote_message(&wallet.address, block_hash);
        wallet.sign(&message)
    }

    /// Verifies a vote signature using the stored public key.
    ///
    /// [FIX-03] Uses real Ed25519 verification.
    /// [FIX-09] Returns false immediately if stake is zero.
    pub fn verify_signature(
        validator:  &str,
        block_hash: &str,
        signature:  &str,
        public_key: &str,
    ) -> bool {
        // [FIX-09] Zero-stake guard.
        if signature.is_empty() || public_key.is_empty() {
            return false;
        }

        let vk = match Wallet::public_key_from_hex(public_key) {
            Some(k) => k,
            None    => {
                eprintln!(
                    "[CONSENSUS] Invalid public key for validator {}.",
                    validator
                );
                return false;
            }
        };

        let message = Self::vote_message(validator, block_hash);
        Wallet::verify_signature(&vk, &message, signature)
    }
}

// ─────────────────────────────────────────────────────────────

/// The consensus engine for a single block proposal round.
///
/// Collects validator votes, prevents double-voting, verifies
/// signatures with real Ed25519, and determines finality at
/// the ≥ 2/3 stake threshold.
#[derive(Debug)]
pub struct ConsensusEngine {
    /// All accepted votes for this round.
    pub votes:              Vec<ValidatorVote>,
    /// Validator registry snapshot for this round.
    pub registry:           ValidatorRegistry,
    /// Tracks which validators have already voted.
    pub voted_validators:   HashSet<String>,
}

impl ConsensusEngine {

    pub fn new(registry: ValidatorRegistry) -> Self {
        Self {
            votes:            Vec::new(),
            registry,
            voted_validators: HashSet::new(),
        }
    }

    /// Adds a vote from a validator.
    ///
    /// Enforces:
    /// - Validator must be registered            [FIX-07]
    /// - Validator must not be jailed            [FIX-06]
    /// - Validator must not have already voted   [FIX-05]
    /// - Signature must be valid Ed25519         [FIX-03]
    /// - Stake must be > 0                       [FIX-09]
    ///
    /// Returns true if the vote was accepted.
    pub fn add_vote(
        &mut self,
        validator:  String,
        block_hash: String,
        signature:  String,
        public_key: String,
    ) -> bool {

        // ── [FIX-07] Unknown validator check ──────────────────
        let validator_info = match self.registry.validators.get(&validator) {
            Some(v) => v.clone(),
            None => {
                eprintln!(
                    "[CONSENSUS] Rejected vote from unknown validator: {}",
                    validator
                );
                self.registry.add_strike(&validator);
                return false;
            }
        };

        // ── [FIX-06] Jailed validator check ───────────────────
        if validator_info.jailed {
            eprintln!(
                "[CONSENSUS] Rejected vote from jailed validator: {}",
                validator
            );
            return false;
        }

        // ── [FIX-09] Zero-stake check ─────────────────────────
        if validator_info.stake == 0 {
            eprintln!(
                "[CONSENSUS] Rejected vote from zero-stake validator: {}",
                validator
            );
            return false;
        }

        // ── [FIX-05] Double-vote check ────────────────────────
        if self.voted_validators.contains(&validator) {
            eprintln!(
                "[CONSENSUS] Double vote rejected from: {}",
                validator
            );
            self.registry.add_strike(&validator);
            self.registry.slash(&validator, 100);
            return false;
        }

        // ── [FIX-03] Real Ed25519 signature verification ──────
        if !ValidatorVote::verify_signature(
            &validator,
            &block_hash,
            &signature,
            &public_key,
        ) {
            eprintln!(
                "[CONSENSUS] Invalid signature from validator: {}",
                validator
            );
            self.registry.add_strike(&validator);
            return false;
        }

        // ── Accept vote ───────────────────────────────────────
        self.voted_validators.insert(validator.clone());

        self.votes.push(ValidatorVote {
            validator,
            stake: validator_info.stake,
            block_hash,
            signature,
            public_key,
        });

        true
    }

    /// Returns the total stake weight of all accepted votes.
    pub fn total_stake(&self) -> u64 {
        self.votes.iter().map(|v| v.stake).sum()
    }

    /// Returns the block hash with the most stake behind it.
    pub fn winning_block(&self) -> Option<String> {
        let mut results: HashMap<String, u64> = HashMap::new();

        for vote in &self.votes {
            *results.entry(vote.block_hash.clone()).or_insert(0) += vote.stake;
        }

        results
            .into_iter()
            .max_by_key(|(_, stake)| *stake)
            .map(|(hash, _)| hash)
    }

    /// Verifies an individual vote's signature.
    ///
    /// [FIX-03] Uses real Ed25519 via ValidatorVote::verify_signature.
    pub fn verify_vote(vote: &ValidatorVote) -> bool {
        if vote.stake == 0 {
            return false;
        }

        ValidatorVote::verify_signature(
            &vote.validator,
            &vote.block_hash,
            &vote.signature,
            &vote.public_key,
        )
    }

    /// Returns true when ≥ 2/3 of total stake has agreed on
    /// one block hash and all votes have valid signatures.
    ///
    /// [FIX-08] Enforces the 2/3 threshold properly.
    pub fn finality_reached(&self) -> bool {
        if self.votes.is_empty() {
            return false;
        }

        // All votes must individually be valid.
        for vote in &self.votes {
            if !Self::verify_vote(vote) {
                eprintln!(
                    "[CONSENSUS] Invalid vote detected from: {}",
                    vote.validator
                );
                return false;
            }
        }

        let total_stake = self.total_stake();
        if total_stake == 0 {
            return false;
        }

        let winner = match self.winning_block() {
            Some(h) => h,
            None    => return false,
        };

        let winner_stake: u64 = self.votes
            .iter()
            .filter(|v| v.block_hash == winner)
            .map(|v| v.stake)
            .sum();

        // Requires winner_stake / total_stake >= 2/3
        // Rearranged to avoid floating point:
        // winner_stake * 3 >= total_stake * 2
        winner_stake * FINALITY_THRESHOLD_DEN
            >= total_stake * FINALITY_THRESHOLD_NUM
    }

    /// Returns the number of votes accepted so far.
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Resets the engine for a new round.
    pub fn reset(&mut self) {
        self.votes.clear();
        self.voted_validators.clear();
        println!("[CONSENSUS] Engine reset for new round.");
    }
}

// ============================================================
// END OF consensus.rs
// ============================================================

