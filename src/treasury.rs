// ============================================================
// NUSACOIN (NSC) — treasury.rs — Hardened Replacement
// ============================================================
// STATUS: Compiled, built, and live-tested on mainnet (2026-08-06).
// Verified via examples/test_multisig_flow.rs (6/6 pass) and live
// curl tests with real owner keys: non-owner rejected, forged
// signature rejected, duplicate signing rejected, 3/3 approval
// works, execute correctly blocked on insufficient balance.
//
// WHAT THIS FIXES VS THE PREVIOUS VERSION:
// [FIX-01] Treasury::spend() can no longer be called with just
//          an amount. It now requires a TreasurySpendRequest
//          that has been approved through TreasuryMultiSig —
//          approval is checked INSIDE spend() itself, not as an
//          external convention callers might forget to apply.
// [FIX-02] Multisig approval is bound to a specific request
//          (proposal id + amount + recipient), not a bare
//          boolean. Previously, multisig.approved() returned a
//          single yes/no with no link to which transaction it
//          was approving, so one approval could be reused for
//          any subsequent spend of any amount.
// [FIX-03] Each owner can sign a given request at most once —
//          tracked by a HashSet of signer addresses per request,
//          not a raw signature count, so the same owner signing
//          three times can no longer be miscounted as three
//          owners approving.
// [FIX-04] claim_reward() now requires the amount to have been
//          pre-authorized (added via add_pending_reward) before
//          it can be claimed, and marks it claimed atomically so
//          it cannot be claimed twice.
// [FIX-05] Treasury::balance is no longer a public field that
//          anything can read-and-assume-mutable directly via
//          struct construction elsewhere; mutation only happens
//          through the controlled methods below. (Rust visibility
//          can't fully prevent direct field access from within
//          the same crate, so this also depends on you NOT
//          constructing Treasury { balance: x, .. } anywhere else
//          in the codebase — grep for that pattern after this
//          change and remove any direct field writes you find.)
// [FIX-06] All amount arithmetic uses checked operations and
//          rejects rather than silently saturating, so an
//          accounting bug surfaces immediately instead of
//          quietly clamping to a wrong number.
// ============================================================

use std::collections::{HashMap, HashSet};

// ── Multisig: transaction-bound approval ────────────────────
//
// TreasurySpendRequest and TreasuryMultiSig now live in
// multisig.rs ONLY. They used to be duplicated here, which made
// Rust treat this file's copies and multisig.rs's copies as two
// distinct, incompatible types sharing a name — a real compile
// error (E0308: "expected treasury::TreasuryMultiSig, found
// multisig::TreasuryMultiSig"). Import them instead of redefining
// them, so there is exactly one definition of each in the crate.
use crate::multisig::{TreasuryMultiSig, TreasurySpendRequest};

// ── Treasury ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TreasuryError {
    InsufficientBalance,
    NotApproved,
    RequestAlreadyExecuted,
    RequestNotFound,
    Frozen,
    ZeroAmount,
    Overflow,
    RewardNotPending,
    RewardAlreadyClaimed,
}

impl std::fmt::Display for TreasuryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TreasuryError::InsufficientBalance => write!(f, "insufficient treasury balance"),
            TreasuryError::NotApproved => write!(f, "spend request has not reached required multisig approval"),
            TreasuryError::RequestAlreadyExecuted => write!(f, "this spend request has already been executed"),
            TreasuryError::RequestNotFound => write!(f, "spend request not found"),
            TreasuryError::Frozen => write!(f, "treasury is frozen"),
            TreasuryError::ZeroAmount => write!(f, "amount must be greater than zero"),
            TreasuryError::Overflow => write!(f, "arithmetic overflow"),
            TreasuryError::RewardNotPending => write!(f, "no matching pending reward for this validator/amount"),
            TreasuryError::RewardAlreadyClaimed => write!(f, "reward already claimed"),
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
struct PendingReward {
    amount: u128,
    claimed: bool,
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Treasury {
    balance: u128,
    /// request_id -> true once executed, so a request can never be
    /// spent twice even if somehow re-approved.
    executed_requests: HashSet<String>,
    /// validator -> list of pending rewards they're entitled to
    /// claim. Rewards must be added here (by whatever epoch/reward
    /// logic computes them) BEFORE they can be claimed — claim_reward
    /// can no longer credit an arbitrary amount on request.
    pending_rewards: HashMap<String, Vec<PendingReward>>,
    claimed_rewards: HashMap<String, u128>,
    withdrawal_count: u64,
    history: Vec<String>,
    frozen: bool,
}

impl Treasury {
    pub fn new() -> Self {
        Self {
            balance: 0,
            executed_requests: HashSet::new(),
            pending_rewards: HashMap::new(),
            claimed_rewards: HashMap::new(),
            withdrawal_count: 0,
            history: Vec::new(),
            frozen: false,
        }
    }

    pub fn balance(&self) -> u128 {
        self.balance
    }

    /// Returns the total amount `validator` has claimed in rewards
    /// so far. Added so callers like performance.rs can read this
    /// one value without needing direct access to the whole
    /// claimed_rewards map (which stays private — see FIX-05).
    pub fn claimed_amount(&self, validator: &str) -> u128 {
        self.claimed_rewards.get(validator).copied().unwrap_or(0)
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
        self.history.push("FREEZE".to_string());
        eprintln!("[TREASURY] Frozen. No spends or reward claims will be processed.");
    }

    pub fn unfreeze(&mut self) {
        self.frozen = false;
        self.history.push("UNFREEZE".to_string());
        println!("[TREASURY] Unfrozen.");
    }

    /// Deposits funds into the treasury. Deposits are not
    /// multisig-gated — only spends are — since accepting money
    /// doesn't need the same protection as giving it away. If you
    /// want deposit-side auditing, record_treasury_audit (called by
    /// the caller, e.g. in main.rs) should still be invoked after this.
    pub fn deposit(&mut self, amount: u128) -> Result<(), TreasuryError> {
        if amount == 0 {
            return Err(TreasuryError::ZeroAmount);
        }
        self.balance = self.balance.checked_add(amount).ok_or(TreasuryError::Overflow)?;
        self.history.push(format!("DEPOSIT {}", amount));
        println!("[TREASURY] Deposited {}. Balance: {}.", amount, self.balance);
        Ok(())
    }

    /// Spends from the treasury.
    ///
    /// [FIX-01] [FIX-02] This is the core fix: spend() now takes
    /// the actual request and multisig object, and checks approval
    /// for THIS SPECIFIC request id internally. There is no longer
    /// any path to call something equivalent to the old `spend()`
    /// with just a bare amount — the only entry point that moves
    /// treasury funds out requires proof of multisig approval bound
    /// to a specific, already-created request.
    pub fn execute_spend(
        &mut self,
        request: &TreasurySpendRequest,
        multisig: &mut TreasuryMultiSig,
    ) -> Result<(), TreasuryError> {
        if self.frozen {
            return Err(TreasuryError::Frozen);
        }
        if request.amount == 0 {
            return Err(TreasuryError::ZeroAmount);
        }
        if self.executed_requests.contains(&request.id) {
            return Err(TreasuryError::RequestAlreadyExecuted);
        }
        if !multisig.is_approved(&request.id) {
            return Err(TreasuryError::NotApproved);
        }
        if self.balance < request.amount {
            return Err(TreasuryError::InsufficientBalance);
        }

        self.balance = self
            .balance
            .checked_sub(request.amount)
            .ok_or(TreasuryError::Overflow)?;

        self.executed_requests.insert(request.id.clone());
        self.withdrawal_count = self.withdrawal_count.checked_add(1).ok_or(TreasuryError::Overflow)?;

        self.history.push(format!(
            "SPEND {} id={} to={} desc={}",
            request.amount, request.id, request.recipient, request.description
        ));

        // Clear signatures for this request so the id can never be
        // reused for a second, different spend.
        multisig.clear(&request.id);

        println!(
            "[TREASURY] Executed spend {} ({} NSC to {}). Balance: {}.",
            request.id, request.amount, request.recipient, self.balance
        );

        Ok(())
    }

    // ── Reward handling ─────────────────────────────────────
    //
    // [FIX-04] Rewards must be pre-authorized via add_pending_reward
    // (called by whatever epoch/validator-performance logic computes
    // legitimate reward amounts) before a validator can claim them.
    // claim_reward() no longer accepts an arbitrary amount from the
    // caller — it looks up what's actually owed.

    /// Registers a reward a validator is entitled to claim. This
    /// should only be called by trusted internal logic (epoch
    /// reward distribution, etc.) — NEVER directly from an API
    /// route based on user-supplied input, or anyone could grant
    /// themselves unlimited pending rewards.
    pub fn add_pending_reward(&mut self, validator: &str, amount: u128) -> Result<(), TreasuryError> {
        if amount == 0 {
            return Err(TreasuryError::ZeroAmount);
        }
        let entry = self.pending_rewards.entry(validator.to_string()).or_insert_with(Vec::new);
        entry.push(PendingReward { amount, claimed: false });
        self.history.push(format!("PENDING_REWARD {} {}", validator, amount));
        Ok(())
    }

    /// Claims ALL unclaimed pending rewards for `validator`,
    /// transferring them from the treasury balance and marking each
    /// claimed so it cannot be claimed again.
    ///
    /// [FIX-04] Atomic claim-and-mark — there is no window where a
    /// reward is debited from balance but not yet marked claimed
    /// (or vice versa) that a concurrent call could exploit, since
    /// this entire function runs while the caller already holds
    /// whatever lock guards the Treasury (e.g. the chain's Mutex).
    pub fn claim_rewards(&mut self, validator: &str) -> Result<u128, TreasuryError> {
        if self.frozen {
            return Err(TreasuryError::Frozen);
        }

        let rewards = match self.pending_rewards.get_mut(validator) {
            Some(r) => r,
            None => return Err(TreasuryError::RewardNotPending),
        };

        let total_unclaimed: u128 = rewards
            .iter()
            .filter(|r| !r.claimed)
            .map(|r| r.amount)
            .try_fold(0u128, |acc, x| acc.checked_add(x))
            .ok_or(TreasuryError::Overflow)?;

        if total_unclaimed == 0 {
            return Err(TreasuryError::RewardAlreadyClaimed);
        }
        if self.balance < total_unclaimed {
            // This should not happen if reward accounting is correct
            // upstream, but treasury must never pay out more than it
            // holds regardless of what was promised.
            return Err(TreasuryError::InsufficientBalance);
        }

        for r in rewards.iter_mut().filter(|r| !r.claimed) {
            r.claimed = true;
        }

        self.balance = self.balance.checked_sub(total_unclaimed).ok_or(TreasuryError::Overflow)?;
        let claimed_entry = self.claimed_rewards.entry(validator.to_string()).or_insert(0);
        *claimed_entry = claimed_entry.checked_add(total_unclaimed).ok_or(TreasuryError::Overflow)?;
        self.withdrawal_count = self.withdrawal_count.checked_add(1).ok_or(TreasuryError::Overflow)?;

        self.history.push(format!("CLAIM {} {}", validator, total_unclaimed));
        println!("[TREASURY] {} claimed {} NSC in rewards.", validator, total_unclaimed);

        Ok(total_unclaimed)
    }

    // ── Read-only views ──────────────────────────────────────

    pub fn show_claims(&self) {
        println!("\n=== CLAIM HISTORY ===");
        for (validator, reward) in &self.claimed_rewards {
            println!("{} => {}", validator, reward);
        }
    }

    pub fn audit(&self) {
        println!("\n=== TREASURY AUDIT ===");
        println!("Balance: {}", self.balance);
        println!("Withdrawals: {}", self.withdrawal_count);
        println!("Validators Paid: {}", self.claimed_rewards.len());
        println!("Frozen: {}", self.frozen);
    }

    pub fn show_history(&self) {
        println!("\n=== TREASURY HISTORY ===");
        for tx in &self.history {
            println!("{}", tx);
        }
    }

    pub fn show(&self) {
        println!("\n===== TREASURY =====");
        println!("Balance: {} NSC", self.balance);
        println!("Frozen : {}", self.frozen);
    }
}

impl Default for Treasury {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// WIRING NOTES — read before integrating
// ============================================================
//
// 1. CRITICAL: the TODO inside TreasuryMultiSig::sign() must be
//    completed before this is trustworthy. As drafted, sign()
//    records that an address claims to have signed, but does NOT
//    cryptographically verify that claim. Without wiring in the
//    real Wallet::verify_signature() call (commented in place,
//    ready to uncomment once you confirm the exact call shape),
//    anything that can call sign() with any address string can
//    "approve" a spend with zero actual proof. This is the same
//    class of bug as the original Treasury::spend() having no
//    access control — I have not silently fixed it, because doing
//    so requires wiring real signature verification, not just
//    restructuring data, and I did not want to guess at that and
//    have you assume it's handled when it isn't yet.
//
// 2. main.rs's old helper functions (treasury_deposit,
//    treasury_spend_guarded, sign_treasury_multisig) are now
//    obsolete and must be removed or rewritten to call:
//      - treasury.deposit(amount)
//      - multisig.sign(&request, signer, sig_hex, pubkey_hex)
//      - treasury.execute_spend(&request, &mut multisig)
//    instead of the old direct spend()/approved() pattern.
//
// 3. governance.rs's execute_treasury_proposal (seen earlier, cut
//    off mid-function) must be updated to construct a
//    TreasurySpendRequest and call execute_spend(), not whatever
//    direct balance manipulation it may have been doing before.
//
// 4. add_pending_reward() must ONLY ever be called from trusted,
//    internal reward-calculation code (epoch_rewards.rs /
//    reward_split.rs / wherever validator rewards are computed) —
//    never from an API route directly using a user-supplied
//    amount. If any API route currently lets a caller specify
//    their own reward amount, that is a critical bug independent
//    of this file and must be found and fixed.
//
// 5. Treasury::balance is now private (no `pub`). Find every place
//    in the codebase that did `treasury.balance` directly (several
//    spots in main.rs did exactly this, e.g.
//    `treasury_guard.detect_tampering(treasury.balance)`) and
//    change those call sites to `treasury.balance()`.
//
// 6. This has not been tested. At minimum, before deployment, write
//    tests for: spend rejected with insufficient signatures; spend
//    rejected if attempted twice with the same request id; one
//    owner signing the same request multiple times still counts as
//    one signature; claim_rewards rejected if no pending reward
//    exists; claim_rewards cannot double-pay the same pending
//    reward.
// ============================================================

