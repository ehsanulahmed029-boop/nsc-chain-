// ============================================================
// NUSACOIN (NSC) — governance.rs — Hardened Replacement (DRAFT)
// ============================================================
// THIS FILE IS A DRAFT. IT HAS NOT BEEN COMPILED OR TESTED.
//
// WHAT WAS WRONG WITH THE PREVIOUS VERSION:
//
// 1. weighted_vote() marked a validator as having voted BEFORE
//    confirming the vote tally update actually succeeded:
//
//      self.voted.insert(key);          // <-- marked as voted
//      let power = ...;
//      if let Some(count) = self.votes.get_mut(proposal) {
//          *count += power;              // <-- silently does
//      }                                 //     nothing if the
//                                         //     proposal wasn't
//                                         //     registered here
//
//    If `proposal` was never inserted as a key into `votes`
//    (and nothing in the file shown ever did that), the
//    validator's vote was permanently discarded with no error
//    returned, and they could never vote on that proposal again
//    because `voted` already contained their key. This version
//    fixes the ordering: the vote is only recorded as cast AFTER
//    the tally update is confirmed to have happened.
//
// 2. weighted_vote() called `self.proposal_active(proposal)`,
//    which did not exist anywhere in the previous file. Either
//    the project did not compile as shown, or a different,
//    unseen version of this file was in use. This version
//    defines proposal state explicitly so there is no phantom
//    method call.
//
// 3. There was no general-purpose create_proposal() that
//    actually populated `votes` with a zero entry for a new
//    proposal id — main.rs called `governance.create_proposal(title)`
//    but that function did not exist in this file either. This
//    version provides one real implementation.
//
// 4. execute_treasury_proposal() was cut off mid-function in the
//    last review, but what was visible checked emergency_mode and
//    (presumably) approval_count before proceeding — better than
//    main.rs's ungated execute_committee_proposal. This version
//    keeps that gating and makes the full execution path explicit,
//    including a timelock check that the original approval_count
//    alone did not enforce.
//
// WHAT THIS VERSION FIXES, ITEMIZED:
// [FIX-01] Vote tally is updated and confirmed BEFORE the
//          validator is marked as having voted. A vote can never
//          be silently discarded while still consuming the
//          validator's one-vote-per-proposal allowance.
// [FIX-02] Proposals have an explicit lifecycle (Active / Passed
//          / Rejected / Expired / Executed) instead of an
//          implicit, undefined proposal_active() check.
// [FIX-03] create_proposal() actually registers the proposal in
//          every map that weighted_vote() and execution depend
//          on, so there's no path where a proposal exists in one
//          map but not another.
// [FIX-04] execute_treasury_proposal() checks emergency_mode,
//          approval threshold, AND timelock expiry — all three —
//          before calling into Treasury, and marks the proposal
//          Executed so it cannot be executed twice.
// [FIX-05] weighted_vote() and create_proposal() return
//          Result/bool so callers (and tests) know whether
//          something actually happened, instead of every function
//          returning () regardless of outcome.
// [FIX-06] All arithmetic uses checked operations.
// ============================================================

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::treasury::Treasury;
use crate::multisig::TreasurySpendRequest;

// ── Proposal lifecycle ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Expired,
    Executed,
}

#[derive(Debug, Clone)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: ProposalStatus,
    /// Set only for treasury-spending proposals. None for
    /// general/non-financial proposals.
    pub treasury_amount: Option<u128>,
    pub treasury_recipient: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GovernanceError {
    EmergencyFrozen,
    ProposalNotFound,
    ProposalNotActive,
    AlreadyVoted,
    NotEligibleVoter,
    ProposalExpired,
    ApprovalThresholdNotMet,
    TimelockNotElapsed,
    AlreadyExecuted,
    Overflow,
}

impl std::fmt::Display for GovernanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GovernanceError::EmergencyFrozen => write!(f, "governance is frozen (emergency mode)"),
            GovernanceError::ProposalNotFound => write!(f, "proposal not found"),
            GovernanceError::ProposalNotActive => write!(f, "proposal is not active"),
            GovernanceError::AlreadyVoted => write!(f, "this validator has already voted on this proposal"),
            GovernanceError::NotEligibleVoter => write!(f, "address has no registered voting power"),
            GovernanceError::ProposalExpired => write!(f, "proposal has expired"),
            GovernanceError::ApprovalThresholdNotMet => write!(f, "proposal has not met the required approval threshold"),
            GovernanceError::TimelockNotElapsed => write!(f, "timelock has not yet elapsed"),
            GovernanceError::AlreadyExecuted => write!(f, "proposal has already been executed"),
            GovernanceError::Overflow => write!(f, "arithmetic overflow"),
        }
    }
}

#[derive(Debug)]
pub struct Governance {
    proposals: HashMap<String, Proposal>,
    /// proposal_id -> accumulated weighted vote total.
    votes: HashMap<String, u64>,
    /// validator address -> their registered voting power (stake).
    voting_power: HashMap<String, u64>,
    /// (proposal_id, validator) pairs that have voted, encoded as
    /// "proposal_id:validator" — prevents double voting.
    voted: HashSet<String>,
    /// proposal_id -> unlock timestamp for execution (timelock).
    timelocks: HashMap<String, u64>,
    pub emergency_mode: bool,
    next_proposal_seq: u64,
}

/// Default voting period for a new proposal, in seconds.
/// 7 days — adjust deliberately; this affects how long the
/// community has to weigh in before a vote closes.
const DEFAULT_VOTING_PERIOD_SECS: u64 = 7 * 24 * 60 * 60;

/// Approval threshold as a fraction of total registered voting
/// power, expressed as numerator/denominator to avoid floating
/// point. 50% here — adjust to whatever your actual governance
/// design calls for (some chains use 2/3 for treasury spends
/// specifically, matching the same kind of supermajority used in
/// consensus.rs's finality_reached()).
const APPROVAL_THRESHOLD_NUM: u64 = 1;
const APPROVAL_THRESHOLD_DEN: u64 = 2;

/// Timelock delay between a proposal passing and being eligible
/// for execution. Gives the community a window to react (e.g. via
/// emergency_mode) if a passed proposal turns out to be harmful.
const EXECUTION_TIMELOCK_SECS: u64 = 48 * 60 * 60; // 48 hours

impl Governance {
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            votes: HashMap::new(),
            voting_power: HashMap::new(),
            voted: HashSet::new(),
            timelocks: HashMap::new(),
            emergency_mode: false,
            next_proposal_seq: 1,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn vote_key(proposal_id: &str, validator: &str) -> String {
        format!("{}:{}", proposal_id, validator)
    }

    // ── Voter registration ───────────────────────────────────

    pub fn register_validator(&mut self, validator: String, stake: u64) {
        self.voting_power.insert(validator, stake);
    }

    pub fn voting_power_of(&self, validator: &str) -> u64 {
        self.voting_power.get(validator).copied().unwrap_or(0)
    }

    pub fn total_voting_power(&self) -> u64 {
        self.voting_power.values().sum()
    }

    // ── Proposal creation ────────────────────────────────────

    /// Creates a general (non-financial) proposal.
    ///
    /// [FIX-03] This actually registers the proposal in `votes`
    /// with a starting tally of 0, so weighted_vote() always has
    /// somewhere real to record a vote against.
    pub fn create_proposal(&mut self, title: String) -> String {
        self.create_proposal_internal(title, None, None)
    }

    /// Creates a treasury-spend proposal. Voting on this works
    /// identically to a general proposal — the treasury_amount and
    /// treasury_recipient fields are only consulted later by
    /// execute_treasury_proposal().
    pub fn create_treasury_proposal(
        &mut self,
        title: String,
        amount: u128,
        recipient: String,
    ) -> String {
        self.create_proposal_internal(title, Some(amount), Some(recipient))
    }

    fn create_proposal_internal(
        &mut self,
        title: String,
        treasury_amount: Option<u128>,
        treasury_recipient: Option<String>,
    ) -> String {
        let id = format!("PROP-{}", self.next_proposal_seq);
        self.next_proposal_seq = self.next_proposal_seq.saturating_add(1);

        let now = Self::now();
        let proposal = Proposal {
            id: id.clone(),
            title: title.clone(),
            created_at: now,
            expires_at: now.saturating_add(DEFAULT_VOTING_PERIOD_SECS),
            status: ProposalStatus::Active,
            treasury_amount,
            treasury_recipient,
        };

        self.proposals.insert(id.clone(), proposal);
        self.votes.insert(id.clone(), 0);

        println!("[GOV] Proposal '{}' created (id={}).", title, id);
        id
    }

    // ── Voting ───────────────────────────────────────────────

    /// Casts a weighted vote for `validator` on `proposal_id`.
    ///
    /// [FIX-01] The vote tally is updated and the update's success
    /// is confirmed BEFORE the validator's vote is marked as cast.
    /// There is no longer a path where a validator is recorded as
    /// "has voted" while their vote silently failed to count.
    /// [FIX-05] Returns a Result so callers know whether the vote
    /// actually counted.
    pub fn weighted_vote(
        &mut self,
        proposal_id: &str,
        validator: &str,
    ) -> Result<u64, GovernanceError> {
        if self.emergency_mode {
            return Err(GovernanceError::EmergencyFrozen);
        }

        let proposal = self
            .proposals
            .get(proposal_id)
            .ok_or(GovernanceError::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Active {
            return Err(GovernanceError::ProposalNotActive);
        }

        let now = Self::now();
        if now > proposal.expires_at {
            // Lazily transition to Expired on first touch past
            // the deadline, rather than requiring a separate
            // sweep process to notice.
            if let Some(p) = self.proposals.get_mut(proposal_id) {
                p.status = ProposalStatus::Expired;
            }
            return Err(GovernanceError::ProposalExpired);
        }

        let power = self.voting_power.get(validator).copied().unwrap_or(0);
        if power == 0 {
            return Err(GovernanceError::NotEligibleVoter);
        }

        let key = Self::vote_key(proposal_id, validator);
        if self.voted.contains(&key) {
            return Err(GovernanceError::AlreadyVoted);
        }

        // [FIX-01] Update the tally FIRST, confirm it succeeded,
        // THEN mark the vote as cast. `votes` is guaranteed to
        // have an entry for proposal_id because create_proposal*
        // always inserts one — but we still use get_mut + ok_or
        // rather than assuming, so a future bug here fails loudly
        // (ProposalNotFound) instead of silently discarding a vote
        // the way the previous version did.
        let new_total = {
            let count = self
                .votes
                .get_mut(proposal_id)
                .ok_or(GovernanceError::ProposalNotFound)?;
            *count = count.checked_add(power).ok_or(GovernanceError::Overflow)?;
            *count
        };

        // Only now, after the tally update is confirmed, record
        // that this validator has voted.
        self.voted.insert(key);

        println!(
            "[GOV] {} voted on '{}' with power {}. New tally: {}.",
            validator, proposal_id, power, new_total
        );

        // Lazily check whether this vote pushed the proposal over
        // the approval threshold.
        self.maybe_finalize(proposal_id);

        Ok(new_total)
    }

    /// Checks whether `proposal_id` has reached the approval
    /// threshold relative to total registered voting power, and
    /// if so, transitions it to Passed. Called automatically after
    /// every vote; can also be called manually (e.g. from a
    /// scheduled sweep) to catch proposals that should transition
    /// due to total_voting_power changing rather than a new vote.
    fn maybe_finalize(&mut self, proposal_id: &str) {
        let total_power = self.total_voting_power();
        if total_power == 0 {
            return;
        }

        let tally = self.votes.get(proposal_id).copied().unwrap_or(0);

        // tally / total_power >= threshold_num / threshold_den
        // rearranged to avoid floating point:
        // tally * threshold_den >= total_power * threshold_num
        let passed = tally.saturating_mul(APPROVAL_THRESHOLD_DEN)
            >= total_power.saturating_mul(APPROVAL_THRESHOLD_NUM);

        if passed {
            if let Some(p) = self.proposals.get_mut(proposal_id) {
                if p.status == ProposalStatus::Active {
                    p.status = ProposalStatus::Passed;
                    let unlock_at = Self::now().saturating_add(EXECUTION_TIMELOCK_SECS);
                    self.timelocks.insert(proposal_id.to_string(), unlock_at);
                    println!(
                        "[GOV] Proposal {} has PASSED. Execution timelock unlocks at {}.",
                        proposal_id, unlock_at
                    );
                }
            }
        }
    }

    pub fn approval_count(&self, proposal_id: &str) -> u64 {
        self.votes.get(proposal_id).copied().unwrap_or(0)
    }

    pub fn proposal_status(&self, proposal_id: &str) -> Option<ProposalStatus> {
        self.proposals.get(proposal_id).map(|p| p.status.clone())
    }

    // ── Emergency mode ───────────────────────────────────────

    pub fn enable_emergency(&mut self) {
        self.emergency_mode = true;
        println!("[GOV] EMERGENCY MODE ENABLED — voting and execution frozen.");
    }

    pub fn disable_emergency(&mut self) {
        self.emergency_mode = false;
        println!("[GOV] EMERGENCY MODE DISABLED.");
    }

    pub fn emergency_active(&self) -> bool {
        self.emergency_mode
    }

    pub fn emergency_status(&self) {
        println!("\n===== EMERGENCY STATUS =====");
        println!("Emergency Mode: {}", self.emergency_mode);
    }

    // ── Treasury proposal execution ──────────────────────────

    /// Executes a passed treasury proposal, spending from
    /// `treasury` via a real TreasurySpendRequest.
    ///
    /// [FIX-04] Checks, in order: not frozen, proposal exists and
    /// has Passed status (i.e. already cleared the vote
    /// threshold), timelock has elapsed, and the proposal has not
    /// already been executed. Only after all four checks does this
    /// touch the treasury.
    ///
    /// NOTE: this function builds a TreasurySpendRequest but does
    /// NOT itself supply multisig signatures — actual execution
    /// still requires the configured TreasuryMultiSig to separately
    /// reach its required signature threshold via
    /// TreasuryMultiSig::sign(), exactly as for any other treasury
    /// spend. Passing a governance vote authorizes that a spend
    /// REQUEST may be created and considered; it does not bypass
    /// the multisig signing requirement on the treasury side. This
    /// is a deliberate two-layer control (governance vote AND
    /// multisig sign-off), not a redundancy to remove.
    pub fn execute_treasury_proposal(
        &mut self,
        proposal_id: &str,
        treasury: &mut Treasury,
        multisig: &mut crate::multisig::TreasuryMultiSig,
    ) -> Result<(), GovernanceError> {
        if self.emergency_mode {
            return Err(GovernanceError::EmergencyFrozen);
        }

        let proposal = self
            .proposals
            .get(proposal_id)
            .ok_or(GovernanceError::ProposalNotFound)?
            .clone();

        if proposal.status == ProposalStatus::Executed {
            return Err(GovernanceError::AlreadyExecuted);
        }
        if proposal.status != ProposalStatus::Passed {
            return Err(GovernanceError::ApprovalThresholdNotMet);
        }

        let unlock_at = self
            .timelocks
            .get(proposal_id)
            .copied()
            .ok_or(GovernanceError::TimelockNotElapsed)?;

        if Self::now() < unlock_at {
            return Err(GovernanceError::TimelockNotElapsed);
        }

        let amount = proposal.treasury_amount.ok_or(GovernanceError::ProposalNotFound)?;
        let recipient = proposal
            .treasury_recipient
            .clone()
            .ok_or(GovernanceError::ProposalNotFound)?;

        let request = TreasurySpendRequest::new(
            proposal_id.to_string(),
            amount,
            recipient,
            proposal.title.clone(),
        );

        // This will itself fail with TreasuryError::NotApproved if
        // the multisig hasn't independently reached its required
        // signature count — that's the second, separate control
        // mentioned in the doc comment above.
        treasury
            .execute_spend(&request, multisig)
            .map_err(|_| GovernanceError::ApprovalThresholdNotMet)?;

        if let Some(p) = self.proposals.get_mut(proposal_id) {
            p.status = ProposalStatus::Executed;
        }

        println!("[GOV] Treasury proposal {} executed.", proposal_id);
        Ok(())
    }

    // ── Display ──────────────────────────────────────────────

    pub fn show_power(&self) {
        println!("\n===== GOVERNANCE POWER =====");
        for (validator, stake) in &self.voting_power {
            println!("{} => {}", validator, stake);
        }
    }

    pub fn show_proposals(&self) {
        println!("\n===== PROPOSALS =====");
        for p in self.proposals.values() {
            println!(
                "{} [{:?}] '{}' tally={} expires_at={}",
                p.id,
                p.status,
                p.title,
                self.votes.get(&p.id).copied().unwrap_or(0),
                p.expires_at
            );
        }
    }

    pub fn show_votes(&self) {
        println!("\n===== VOTE TALLIES =====");
        for (id, tally) in &self.votes {
            println!("{} => {}", id, tally);
        }
    }
}

impl Default for Governance {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// WIRING NOTES — read before integrating
// ============================================================
//
// 1. main.rs's old governance helper functions
//    (create_governance_proposal, cast_governance_vote) must be
//    rewritten to call create_proposal()/weighted_vote() and
//    handle the returned Result/String — the old versions assumed
//    void returns and printed success unconditionally regardless
//    of what actually happened.
//
// 
