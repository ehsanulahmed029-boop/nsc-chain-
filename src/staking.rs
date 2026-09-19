// ============================================================
// NUSACOIN (NSC) — staking.rs — Hardened, wired to real balances
// ============================================================
// [FIX-01] stake() requires a real balance debit through
//          crate::amm::BalanceLedger (same trait already
//          implemented on Blockchain in chain.rs) before recording
//          any stake increase. You cannot stake what you don't have.
// [FIX-02] Staking is ADDITIVE — calling stake() again increases
//          the validator's existing stake, it never overwrites it.
// [FIX-03] unstake() moves stake into an unbonding queue with a
//          delay before funds are actually returned, matching
//          standard PoS practice (prevents a validator from
//          misbehaving and then instantly withdrawing before a
//          slash can be applied).
// [FIX-04] claim_unbonded() releases funds back to the real balance
//          only after the unbonding period has elapsed, and only
//          once per unbonding entry. Rollback on credit failure only
//          unsets the specific entries flipped in THIS call (tracked
//          via a local id list), not all matured-looking entries.
// [FIX-05] slash_stake() reduces a validator's real recorded stake.
//          The caller decides where slashed funds go (this function
//          does not move funds anywhere itself).
// [FIX-06] All arithmetic uses checked operations.
// [FIX-07] (2026-08-11 wiring pass) All amounts/ids/timestamps use
//          u128, matching chain.rs's balances: HashMap<String, u128>.
//          Reuses crate::amm::BalanceLedger (already implemented for
//          Blockchain) instead of redefining a separate trait.
// ============================================================

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::amm::BalanceLedger;

// ── Constants ────────────────────────────────────────────────

/// How long, in seconds, staked funds are held after unstake()
/// before they can actually be withdrawn via claim_unbonded().
/// This window exists so a validator who misbehaves can still be
/// slashed before they can pull their stake out. 7 days, matching
/// common PoS unbonding periods — adjust deliberately, not
/// casually, since shortening it weakens slashing's deterrent
/// effect and lengthening it locks up validator funds longer.
pub const UNBONDING_PERIOD_SECS: u128 = 7 * 24 * 60 * 60;

/// Minimum stake required to be considered an active validator.
/// Adjust to match your actual tokenomics decision — this is a
/// placeholder, not a number asserted to be correct for NSC.
pub const MIN_VALIDATOR_STAKE: u128 = 1_000;

#[derive(Debug, Clone, PartialEq)]
pub enum StakingError {
    ZeroAmount,
    InsufficientBalance,
    InsufficientStake,
    BalanceUpdateFailed,
    NoSuchUnbondingEntry,
    UnbondingNotYetMature,
    AlreadyClaimed,
    Overflow,
}

impl std::fmt::Display for StakingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StakingError::ZeroAmount => write!(f, "amount must be greater than zero"),
            StakingError::InsufficientBalance => write!(f, "insufficient balance to stake this amount"),
            StakingError::InsufficientStake => write!(f, "validator does not have this much staked"),
            StakingError::BalanceUpdateFailed => write!(f, "failed to update balance"),
            StakingError::NoSuchUnbondingEntry => write!(f, "no matching unbonding entry"),
            StakingError::UnbondingNotYetMature => write!(f, "unbonding period has not yet elapsed"),
            StakingError::AlreadyClaimed => write!(f, "this unbonding entry has already been claimed"),
            StakingError::Overflow => write!(f, "arithmetic overflow"),
        }
    }
}

/// A single staking-related bookkeeping mutation, recorded so the
/// internal Staking ledger (validators + unbonding queue) can be
/// independently replayed during L1 fork-choice resolution, since
/// it is not a simple balance map and cannot be captured by the
/// generic block::BalanceOp model. Deliberately does NOT record any
/// NSC balance change -- that side of stake/unstake/claim is
/// already covered separately via chain.record_op("nsc", ...), so
/// replaying StakingOp never double-applies a balance change.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StakingOp {
    Stake { address: String, amount: u128 },
    Unstake { address: String, amount: u128, unbonding_id: u128, unlock_at: u128 },
    Claim { address: String, unbonding_ids: Vec<u128> },
    Slash { address: String, amount: u128 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UnbondingEntry {
    id: u128,
    amount: u128,
    unlock_at: u128,
    claimed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Staking {
    /// validator address -> currently active (bonded) stake.
    validators: HashMap<String, u128>,
    /// validator address -> their unbonding entries awaiting the
    /// unbonding period to elapse.
    unbonding: HashMap<String, Vec<UnbondingEntry>>,
    next_unbonding_id: u128,
    #[serde(default)]
    pub pending_ops: Vec<StakingOp>,
}

impl Staking {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            unbonding: HashMap::new(),
            next_unbonding_id: 1,
            pending_ops: Vec::new(),
        }
    }

    /// Applies a previously-recorded StakingOp during L1 fork-choice
    /// replay. Performs ONLY the staking bookkeeping mutation -- never
    /// touches any NSC balance, since that side is already covered by
    /// separate "nsc" BalanceOp replay. Idempotent-safe ordering: ops
    /// must be applied in the exact original block order.
        pub fn apply_op(&mut self, op: &StakingOp) {
            match op {
                StakingOp::Stake { address, amount } => {
                    let entry = self.validators.entry(address.clone()).or_insert(0);
                    *entry = entry.saturating_add(*amount);
                }
                StakingOp::Unstake { address, amount, unbonding_id, unlock_at } => {
                    let current = self.validators.get(address).copied().unwrap_or(0);
                    let remaining = current.saturating_sub(*amount);
                    if remaining == 0 {
                        self.validators.remove(address);
                    } else {
                        self.validators.insert(address.clone(), remaining);
                    }
                    let entries = self.unbonding.entry(address.clone()).or_insert_with(Vec::new);
                    entries.push(UnbondingEntry { id: *unbonding_id, amount: *amount, unlock_at: *unlock_at, claimed: false });
                    if *unbonding_id >= self.next_unbonding_id {
                        self.next_unbonding_id = *unbonding_id + 1;
                    }
                }
                StakingOp::Claim { address, unbonding_ids } => {
                    if let Some(entries) = self.unbonding.get_mut(address) {
                        for entry in entries.iter_mut() {
                            if unbonding_ids.contains(&entry.id) {
                                entry.claimed = true;
                            }
                        }
                        entries.retain(|e| !e.claimed);
                    }
                }
                StakingOp::Slash { address, amount } => {
                    let current = self.validators.get(address).copied().unwrap_or(0);
                    let slashed = (*amount).min(current);
                    let remaining = current - slashed;
                    if remaining == 0 {
                        self.validators.remove(address);
                    } else {
                        self.validators.insert(address.clone(), remaining);
                    }
                }
            }
        }

    fn now() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u128
    }

    /// Stakes `amount` NSC for `address`, debiting their real
    /// balance first.
    ///
    /// [FIX-01] [FIX-02] This is additive — repeated calls
    /// increase total stake, they never overwrite it. The debit
    /// happens before any stake bookkeeping changes, so a failed
    /// debit leaves everything unchanged.
    pub fn stake<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        address: &str,
        amount: u128,
    ) -> Result<u128, StakingError> {
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }

        if ledger.nsc_balance(address) < amount {
            return Err(StakingError::InsufficientBalance);
        }

        if !ledger.debit_nsc(address, amount) {
            return Err(StakingError::BalanceUpdateFailed);
        }

        let entry = self.validators.entry(address.to_string()).or_insert(0);
        *entry = entry.checked_add(amount).ok_or(StakingError::Overflow)?;
        let new_total = *entry;
        self.pending_ops.push(StakingOp::Stake { address: address.to_string(), amount });

        println!(
            "[STAKING] {} staked {} NSC. Total bonded stake: {}.",
            address, amount, new_total
        );

        Ok(new_total)
    }

    /// Begins unstaking `amount` from `address`'s active stake.
    /// The funds are NOT returned immediately — they move into an
    /// unbonding queue and become claimable after
    /// UNBONDING_PERIOD_SECS has elapsed.
    ///
    /// [FIX-03] This function did not exist before at all.
    pub fn unstake(&mut self, address: &str, amount: u128) -> Result<u128, StakingError> {
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }

        let current = self.validators.get(address).copied().unwrap_or(0);
        if current < amount {
            return Err(StakingError::InsufficientStake);
        }

        let remaining = current - amount;
        if remaining == 0 {
            self.validators.remove(address);
        } else {
            self.validators.insert(address.to_string(), remaining);
        }

        let id = self.next_unbonding_id;
        self.next_unbonding_id = self.next_unbonding_id.checked_add(1).ok_or(StakingError::Overflow)?;

        let unlock_at = Self::now().checked_add(UNBONDING_PERIOD_SECS).ok_or(StakingError::Overflow)?;

        let entries = self.unbonding.entry(address.to_string()).or_insert_with(Vec::new);
        entries.push(UnbondingEntry { id, amount, unlock_at, claimed: false });

        println!(
            "[STAKING] {} began unstaking {} NSC (unbonding entry #{}, unlocks at {}).",
            address, amount, id, unlock_at
        );

        self.pending_ops.push(StakingOp::Unstake { address: address.to_string(), amount, unbonding_id: id, unlock_at });
        Ok(id)
    }

    /// Claims any matured (past unbonding period) unbonding
    /// entries for `address`, crediting the real balance.
    ///
    /// [FIX-04] Funds are only released after the unbonding
    /// period has elapsed, and each entry can only be claimed
    /// once. If the credit fails, only the entries flipped to
    /// `claimed = true` during THIS call (tracked in
    /// `flipped_this_call`) are rolled back — entries claimed in
    /// an earlier call are left untouched.
    pub fn claim_unbonded<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        address: &str,
    ) -> Result<u128, StakingError> {
        let now = Self::now();

        let entries = self
            .unbonding
            .get_mut(address)
            .ok_or(StakingError::NoSuchUnbondingEntry)?;

        let mut total: u128 = 0;
        let mut flipped_this_call: Vec<u128> = Vec::new();

        for entry in entries.iter_mut() {
            if entry.claimed {
                continue;
            }
            if entry.unlock_at > now {
                continue;
            }
            total = total.checked_add(entry.amount).ok_or(StakingError::Overflow)?;
            entry.claimed = true;
            flipped_this_call.push(entry.id);
        }

        if flipped_this_call.is_empty() {
            return Err(StakingError::UnbondingNotYetMature);
        }
        if total == 0 {
            return Err(StakingError::AlreadyClaimed);
        }

        if !ledger.credit_nsc(address, total) {
            // [FIX-04] Roll back only the entries we flipped in THIS
            // call, identified by id — entries claimed in a previous
            // call are left untouched.
            for entry in entries.iter_mut() {
                if flipped_this_call.contains(&entry.id) {
                    entry.claimed = false;
                }
            }
            return Err(StakingError::BalanceUpdateFailed);
        }

        // Clean up fully-claimed entries to keep the vector small.
        entries.retain(|e| !e.claimed);

        println!("[STAKING] {} claimed {} NSC from matured unbonding entries.", address, total);

        self.pending_ops.push(StakingOp::Claim { address: address.to_string(), unbonding_ids: flipped_this_call.clone() });
        Ok(total)
    }

    /// Reduces a validator's active stake as a penalty.
    ///
    /// [FIX-05] Gives slashing logic a real lever to pull: an
    /// actual reduction in bonded stake, not just a separate
    /// penalty counter.
    ///
    /// The slashed amount is NOT returned to the validator and
    /// NOT credited anywhere by this function — the caller must
    /// decide where slashed funds go and call ledger.credit_nsc()
    /// on the appropriate destination itself.
    pub fn slash_stake(&mut self, address: &str, amount: u128) -> Result<u128, StakingError> {
        // [FIX-09] amount == 0 is a caller error, not a statement about
        // the validator's stake — report it as ZeroAmount so callers
        // can distinguish "you asked to slash nothing" from "this
        // validator has nothing left to slash".
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }

        let current = self.validators.get(address).copied().unwrap_or(0);
        let slashed = amount.min(current);

        if slashed == 0 {
            return Err(StakingError::InsufficientStake);
        }

        let remaining = current - slashed;
        if remaining == 0 {
            self.validators.remove(address);
        } else {
            self.validators.insert(address.to_string(), remaining);
        }

        eprintln!(
            "[STAKING] SLASHED {} NSC from {}. Remaining stake: {}.",
            slashed, address, remaining
        );

        self.pending_ops.push(StakingOp::Slash { address: address.to_string(), amount: slashed });
        Ok(slashed)
    }

    // ── Selection / queries ────────────────────────────────────

    /// Returns the validator with the highest active stake, if
    /// any meet MIN_VALIDATOR_STAKE.
    pub fn select_validator(&self) -> Option<String> {
        self.validators
            .iter()
            .filter(|&(_, &stake)| stake >= MIN_VALIDATOR_STAKE)
            .max_by_key(|&(_, &stake)| stake)
            .map(|(addr, _)| addr.clone())
    }

    pub fn stake_of(&self, address: &str) -> u128 {
        self.validators.get(address).copied().unwrap_or(0)
    }

    pub fn is_active_validator(&self, address: &str) -> bool {
        self.stake_of(address) >= MIN_VALIDATOR_STAKE
    }

    pub fn total_staked(&self) -> u128 {
        // [FIX-08] Checked sum instead of `.sum()` — a raw sum silently
        // wraps on overflow in release builds. Saturating here is the
        // right failure mode for a read-only display total: it cannot
        // corrupt any real balance (this function never mutates state),
        // and clamping to u128::MAX is a more honest signal that
        // something is very wrong than silently wrapping to a small
        // number would be.
        self.validators
            .values()
            .fold(0u128, |acc, &stake| acc.saturating_add(stake))
    }

    pub fn validator_count(&self) -> usize {
        self.validators.iter().filter(|&(_, &s)| s >= MIN_VALIDATOR_STAKE).count()
    }

    pub fn show_validators(&self) {
        println!("\n===== VALIDATORS =====");
        for (addr, amount) in &self.validators {
            let active = if *amount >= MIN_VALIDATOR_STAKE { "active" } else { "below minimum" };
            println!("{} => {} NSC ({})", addr, amount, active);
        }
    }

    pub fn show_unbonding(&self, address: &str) {
        println!("\n===== UNBONDING: {} =====", address);
        match self.unbonding.get(address) {
            Some(entries) => {
                for e in entries {
                    println!(
                        "  #{} amount={} unlock_at={} claimed={}",
                        e.id, e.amount, e.unlock_at, e.claimed
                    );
                }
            }
            None => println!("  (none)"),
        }
    }
}

impl Default for Staking {
    fn default() -> Self {
        Self::new()
    }
}
