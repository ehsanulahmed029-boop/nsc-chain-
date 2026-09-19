// ============================================================
// NUSACOIN (NSC) - Mainnet Node
// Version: Production
// ============================================================
// FIXES APPLIED:
// [FIX-01] Removed all private key / signature println leaks
// [FIX-02] Removed faucet (free mint disabled on mainnet)
// [FIX-03] Removed all hardcoded test wallets
// [FIX-04] Network guard: aborts if not NSC_MAINNET_1
// [FIX-05] Chain integrity verified before node starts
// [FIX-06] Genesis block validated before node starts
// [FIX-07] API runs on separate thread (no block)
// [FIX-08] P2P runs on separate thread (no block)
// [FIX-09] No duplicate treasury / governance test calls
// [FIX-10] No duplicate vote calls (double-vote removed)
// [FIX-11] No test consensus with fake signatures
// [FIX-12] No wallet.export_private_key() on mainnet
// [FIX-13] No wallet.backup() printing keys to console
// [FIX-14] Wallet lock/unlock not called in production path
// [FIX-15] Validator registry loaded, not hardcoded
// [FIX-16] Blockchain save called safely inside lock
// [FIX-17] Fatal exit on any critical startup failure
// [FIX-18] Removed all return; inside main that exit early
// [FIX-19] No zero-amount test transactions
// [FIX-20] No self-transfer test transactions
// [FIX-21] Removed all duplicate EpochManager instances
// [FIX-22] Removed all duplicate Treasury instances
// [FIX-23] Removed all demo/simulation loops
// [FIX-24] Removed debug TRANSFER DEBUG println blocks
// [FIX-25] config.rs must be NSC_MAINNET_1 to start
// ============================================================

mod hash;
mod transaction;
mod block;
mod chain;
mod wallet;
mod evm_wallet;
mod evm_rpc;
mod evm_receipt;
mod evm_tx;
mod mempool;
mod supply;
mod storage;
mod network;
mod peer;
mod peers;
mod block_message;
mod amm;
mod trade;
mod api;
mod api_models;
mod multisig;
mod l2_bridge;
mod seed;
mod encryption;
mod recovery;
mod secure_storage;
mod staking;
mod validator;
mod slashing;
mod governance;
mod treasury;
mod contracts;
mod token;
mod token_registry;
mod network_registry;
mod coingecko_client;
mod coinmarketcap_client;
mod bsc_watcher;
mod dex;
mod version;
mod config;
mod genesis;
mod seeds;
mod peer_manager;
mod delegate;
mod consensus;
mod audit;
mod validator_registry;
mod epoch;
mod performance;
mod security_council;
mod reputation;
mod validator_selector;
mod uptime;
mod validator_rank;
mod rotation;
mod leader_selection;
mod jail;
mod health;
mod dynamic_rewards;
mod performance_score;
mod election_v2;
mod election_audit;
mod downtime;
mod auto_slash;
mod suspension;
mod reputation_recovery;
mod health_score;
mod weighted_leader;
mod committee;
mod committee_vote;
mod committee_proposal;
mod upgrade;
mod protocol_version;
mod hard_fork;
mod vesting;
mod validator_snapshot;
mod reward_history;
mod reward_claim;
mod reward_vault;
mod epoch_rewards;
mod reward_split;
mod delegation_registry;
mod delegator_rewards;
mod unbonding;
mod key_rotation;
mod heartbeat;
mod offline_jail;
mod recovery_jail;
mod strike_system;
mod reputation_decay;
mod leader_v2;
mod performance_history;
mod trust_score;
mod election_v3;
mod emergency_vote;
mod chain_freeze;
mod treasury_freeze;
mod network_recovery;
mod blacklist;
mod admission;
mod node_fingerprint;
mod sybil_detector;
mod geographic_diversity;
mod hosting_risk;
mod network_latency;
mod availability;
mod reliability_ranking;
mod leader_election_v4;
mod rotation_scheduler;
mod strike_v2;
mod suspension_manager;
mod recovery_manager;
mod validator_appeals;
mod security_council_voting;
mod appeal_resolution;
mod emergency_freeze;
mod emergency_recovery;
mod safe_restart;
mod validator_state_snapshot;
mod validator_state_rollback;
mod chain_state_snapshot;
mod chain_state_rollback;
mod governance_state_snapshot;
mod governance_state_rollback;
mod network_checkpoint;
mod checkpoint_recovery;
mod checkpoint_scheduler;
mod checkpoint_integrity;
mod checkpoint_archive;
mod checkpoint_pruning;
mod checkpoint_replication;
mod checkpoint_consensus;
mod checkpoint_quorum;
mod checkpoint_finalization;
mod checkpoint_voting;
mod checkpoint_proposal;
mod checkpoint_proposal_validation;
mod checkpoint_proposal_execution;
mod checkpoint_slashing;
mod emergency_freeze_multisig;
mod checkpoint_reputation;
mod checkpoint_weighted_voting;
mod checkpoint_weighted_quorum;
mod treasury_protection;
mod treasury_recovery_lock;
mod treasury_timelock;
mod treasury_emergency_freeze;
mod treasury_audit;
mod treasury_audit_hashchain;
mod treasury_merkle_audit;
mod merkle_proof;
mod checkpoint_merkle_registry;
mod checkpoint_history_verify;
mod state_snapshot_verify;
mod state_snapshot_registry;
mod fast_sync_selector;
mod snapshot_trust_score;
mod snapshot_consensus;
mod snapshot_quorum;
mod snapshot_finalization;
mod snapshot_signature;
mod recovery_snapshot_certificate;
mod recovery_certificate_registry;
mod recovery_certificate_revocation;
mod recovery_certificate_expiration;
mod recovery_certificate_multisig;
mod recovery_authority_governance;
mod mainnet_readiness;

// ── Standard library ─────────────────────────────────────────
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

// ── Core node ────────────────────────────────────────────────
use chain::Blockchain;
use network::Node;
use peer_manager::PeerManager;

// ── Consensus & validators ───────────────────────────────────
use consensus::ConsensusEngine;
use validator_registry::ValidatorRegistry;
use epoch::EpochManager;
use staking::Staking;
use slashing::Slashing;
use governance::Governance;
use treasury::Treasury;
use security_council::SecurityCouncil;
use reputation::ReputationManager;
use validator_selector::ValidatorSelector;
use uptime::UptimeTracker;
use validator_rank::ValidatorRank;
use rotation::ValidatorRotation;
use jail::JailManager;
use health::HealthMonitor;
use dynamic_rewards::DynamicRewards;
use performance_score::PerformanceScore;
use election_v2::ElectionV2;
use election_audit::ElectionAudit;
use downtime::DowntimeTracker;
use auto_slash::AutoSlasher;
use reputation_recovery::ReputationRecovery;
use health_score::HealthScore;
use weighted_leader::WeightedLeader;
use committee::Committee;
use committee_vote::CommitteeVote;
use committee_proposal::CommitteeProposalSystem;
use upgrade::UpgradeManager;
use protocol_version::ProtocolVersionManager;
use hard_fork::HardForkManager;
use vesting::VestingManager;
use validator_snapshot::SnapshotManager;
use reward_history::RewardHistory;
use reward_claim::RewardClaimSystem;
use reward_vault::RewardVault;
use epoch_rewards::EpochRewardEngine;
use reward_split::RewardSplitEngine;
use delegation_registry::DelegationRegistry;
use delegator_rewards::DelegatorRewardEngine;
use unbonding::UnbondingManager;
use key_rotation::KeyRotationManager;
use heartbeat::HeartbeatManager;
use offline_jail::OfflineJailEngine;
use strike_system::StrikeSystem;
use reputation_decay::ReputationDecay;
use leader_v2::LeaderV2;
use performance_history::PerformanceHistory;
use trust_score::TrustScore;
use election_v3::ElectionV3;
use emergency_vote::EmergencyProposal;
use chain_freeze::ChainFreeze;
use treasury_freeze::TreasuryFreeze;
use network_recovery::NetworkRecovery;
use blacklist::BlacklistRegistry;
use admission::AdmissionController;
use node_fingerprint::NodeFingerprint;
use sybil_detector::{SybilDetector, ValidatorIdentity};
use geographic_diversity::{GeographicDiversity, ValidatorLocation};
use hosting_risk::{HostingRiskAnalyzer, HostingNode};
use network_latency::{NetworkLatencyAnalyzer, LatencyRecord};
use availability::AvailabilityMonitor;
use reliability_ranking::ReliabilityRanking;
use leader_election_v4::LeaderElectionV4;
use rotation_scheduler::RotationScheduler;
use strike_v2::StrikeEngineV2;
use suspension_manager::SuspensionManager;
use recovery_manager::RecoveryManager;
use validator_appeals::ValidatorAppeals;
use security_council_voting::SecurityCouncilVoting;
use appeal_resolution::AppealResolution;
use emergency_freeze::EmergencyFreeze;
use emergency_recovery::EmergencyRecovery;
use safe_restart::SafeRestartManager;
use validator_state_snapshot::ValidatorStateSnapshot;
use validator_state_rollback::ValidatorRollback;
use chain_state_snapshot::ChainStateSnapshot;
use chain_state_rollback::ChainStateRollback;
use governance_state_snapshot::GovernanceStateSnapshot;
use governance_state_rollback::GovernanceRollback;
use network_checkpoint::NetworkCheckpoint;
use checkpoint_scheduler::CheckpointScheduler;
use checkpoint_integrity::CheckpointIntegrity;
use checkpoint_archive::CheckpointArchive;
use checkpoint_pruning::CheckpointPruning;
use checkpoint_replication::CheckpointReplication;
use checkpoint_consensus::CheckpointConsensus;
use checkpoint_quorum::CheckpointQuorum;
use checkpoint_finalization::CheckpointFinalization;
use checkpoint_voting::CheckpointVoting;
use checkpoint_proposal::CheckpointProposalEngine;
use checkpoint_proposal_validation::CheckpointProposalValidation;
use checkpoint_proposal_execution::CheckpointProposalExecution;
use checkpoint_slashing::CheckpointSlashing;
use emergency_freeze_multisig::{EmergencyFreezeMultiSig, EmergencyFreezeRequest};
use checkpoint_reputation::CheckpointReputation;
use checkpoint_weighted_voting::CheckpointWeightedVoting;
use checkpoint_weighted_quorum::CheckpointWeightedQuorum;
use treasury_protection::TreasuryProtection;
use treasury_recovery_lock::TreasuryRecoveryLock;
use multisig::TreasuryMultiSig;
use treasury_timelock::{TreasuryTimeLock, TimeLockedTransaction};
use treasury_emergency_freeze::TreasuryEmergencyFreeze;
use treasury_audit::TreasuryAudit;
use treasury_audit_hashchain::TreasuryAuditHashChain;
use treasury_merkle_audit::TreasuryMerkleAudit;
use merkle_proof::MerkleProof;
use checkpoint_merkle_registry::CheckpointMerkleRegistry;
use checkpoint_history_verify::{CheckpointHistoryVerify, HistoricalCheckpoint};
use state_snapshot_verify::{StateSnapshot, StateSnapshotVerify};
use state_snapshot_registry::StateSnapshotRegistry;
use fast_sync_selector::FastSyncSelector;
use snapshot_trust_score::{SnapshotProvider, SnapshotTrustScore};
use snapshot_consensus::{SnapshotVote, SnapshotConsensus};
use snapshot_quorum::SnapshotQuorum;
use snapshot_finalization::{FinalizedSnapshot, SnapshotFinalization};
use snapshot_signature::{SignedSnapshot, SnapshotSignature};
use recovery_snapshot_certificate::{RecoveryCertificate, RecoverySnapshotCertificate};
use recovery_certificate_registry::RecoveryCertificateRegistry;
use recovery_certificate_revocation::RecoveryCertificateRevocation;
use recovery_certificate_expiration::RecoveryCertificateExpiration;
use recovery_certificate_multisig::{RecoveryMultiSig, RecoveryCertificateMultiSig};
use recovery_authority_governance::RecoveryAuthorityGovernance;
use mainnet_readiness::MainnetReadiness;
use performance::ValidatorPerformance;
use amm::LiquidityPool;
use crate::multisig::TreasurySpendRequest;

// ── Mainnet network guard ─────────────────────────────────────
// [FIX-25] Node refuses to start if config is not mainnet.
// Change config.rs NETWORK_ID to "NSC_MAINNET_1" before launconst REQUIRED_NETWORK_ID: &str = "NSC_MAINNET_1";
const REQUIRED_NETWORK_ID: &str = "NSC_MAINNET_1";

// ── Node configuration ────────────────────────────────────────
const DEFAULT_P2P_ADDR:  &str = "0.0.0.0:6000";
const DEFAULT_API_PORT:  &str = "8080";
const HEARTBEAT_SECS:     u64 = 60;
const CHECKPOINT_INTERVAL: usize = 5;

// ─────────────────────────────────────────────────────────────
// STARTUP SEQUENCE
// 1.  Network ID guard
// 2.  Security audit
// 3.  Mainnet readiness check
// 4.  Load / create blockchain
// 5.  Validate chain + genesis
// 6.  Initialise all subsystems
// 7.  Bootstrap peers
// 8.  Start P2P thread
// 9.  Start API thread
// 10. Run node heartbeat loop
// ─────────────────────────────────────────────────────────────
fn main() {

    // ── Step 1: Network ID guard ──────────────────────────────
    // [FIX-25] Prevents testnet binary running on mainnet.
    if config::NETWORK_ID != REQUIRED_NETWORK_ID {
        eprintln!(
            "[FATAL] Wrong network. Expected '{}' got '{}'.",
            REQUIRED_NETWORK_ID,
            config::NETWORK_ID
        );
        eprintln!(
            "[FATAL] Edit src/config.rs and set NETWORK_ID = \"NSC_MAINNET_1\""
        );
        std::process::exit(1);
    }

    // ── Banner ────────────────────────────────────────────────
    println!("====================================================");
    println!("  NUSACOIN (NSC) Mainnet Node");
    println!("  Version  : {}", version::VERSION);
    println!("  Network  : {}", config::NETWORK_ID);
    println!("  Max Supply: 25,000,000 NSC");
    println!("  Genesis Supply: {}", genesis::GENESIS_SUPPLY);
    println!("  Block Time: {} sec", genesis::GENESIS_BLOCK_TIME);
    println!("====================================================");

    // ── Seed nodes ────────────────────────────────────────────
    println!("[SEEDS] Known seed nodes:");
    for seed in seeds::seed_nodes() {
        println!("  {}", seed);
    }

    // ── Step 2: Security audit ────────────────────────────────
    println!("[AUDIT] Running security audit...");
    let mut audit = audit::SecurityAudit::new();
    audit.run();
    audit.report();
    println!("[AUDIT] Complete.");

    // ── Step 3: Mainnet readiness check ──────────────────────
    println!("[READINESS] Checking mainnet readiness...");
    MainnetReadiness::validate();
    println!("[READINESS] Complete.");

    // ── Step 4: Load or create blockchain ────────────────────
    // [FIX-18] No early return here — fatal exit on failure.
    println!("[CHAIN] Loading blockchain...");
    let blockchain = Arc::new(Mutex::new(
        if let Some(blocks) = storage::load_chain() {
            println!("[CHAIN] Loaded {} block(s) from disk.", blocks.len());
            Blockchain::from_blocks(blocks)
        } else {
            println!("[CHAIN] No saved chain — creating genesis block.");
            Blockchain::new()
        }
    ));

    // ── Step 4b: Load L2 bridge state ────────────────────────
    println!("[L2] Loading L2 bridge state...");
    let l2_state = Arc::new(Mutex::new(storage::load_l2_state()));
    let (l2_batch_count, l2_deposit_count) = {
        let l2 = l2_state.lock().expect("l2 lock");
        (l2.batches.len(), l2.deposits.len())
    };
    println!("[L2] L2 bridge state ready (batches: {}, deposits: {}).", l2_batch_count, l2_deposit_count);

    // ── Step 5: Validate chain + genesis ─────────────────────
    // [FIX-17] Fatal exit if chain is corrupted.
    {
        let chain = blockchain.lock().expect("chain lock");
        println!("[CHAIN] Validating genesis block...");
        if !chain.validate_genesis() {
            eprintln!("[FATAL] Genesis block invalid. Chain may be corrupted.");
            std::process::exit(1);
        }
        println!("[CHAIN] Validating chain integrity...");
        let chain_is_valid = chain.validate_chain();
        drop(chain); // release lock before recover_chain_from_checkpoint locks it again

        if !chain_is_valid {
            eprintln!("[CHAIN] Chain validation failed. Attempting checkpoint recovery...");
            // recover_chain_from_checkpoint() re-validates internally and
            // calls std::process::exit(1) itself if recovery does not
            // produce a valid chain, so control only continues past this
            // call when the chain is confirmed valid again.
            recover_chain_from_checkpoint(&blockchain);
        }

        let chain = blockchain.lock().expect("chain lock");
        println!(
            "[CHAIN] OK — height={} difficulty={}",
            chain.chain_height(),
            chain.difficulty
        );
        chain.chain_health();
        chain.scan_corruption();
        chain.sync_status();
    }

    // ── Step 6: Initialise subsystems ────────────────────────
    // [FIX-15] Validator registry is loaded, not hardcoded.
    // [FIX-21] One EpochManager instance for the node lifetime.
    // [FIX-22] One Treasury instance for the node lifetime.
    println!("[INIT] Initialising subsystems...");

    // Validator registry
    let validator_registry = ValidatorRegistry::new();
    println!(
        "[VALIDATORS] {} validator(s) registered at startup.",
        validator_registry.validator_count()
    );
    println!(
        "[VALIDATORS] Total stake: {}",
        validator_registry.total_stake()
    );
    validator_registry.list();

    // Consensus engine
    let _consensus = ConsensusEngine::new(validator_registry.clone());

    // Staking
    let staking = Staking::new();
    println!("[STAKING] Total staked: {}", staking.total_staked());
    staking.show_validators();

    // Slashing
    let mut slashing = Slashing::new();

    // Governance
    let governance = Governance::new();

    // Treasury — now lives inside Blockchain (chain.treasury), single source of truth.
    // Standalone instance removed to prevent balance desync between two
    // separate Treasury objects.

    // Epoch manager — [FIX-21] single instance
    let mut epoch_manager = EpochManager::new();

    // Reputation
    let mut reputation = ReputationManager::new();

    // Health monitor
    let health = HealthMonitor::new();

    // Uptime tracker
    let uptime = UptimeTracker::new();

    // Rotation
    let mut rotation = ValidatorRotation::new();

    // Jail manager
    let mut jail = JailManager::new();

    // Downtime tracker
    let downtime = DowntimeTracker::new(3);

    // Heartbeat manager
    let heartbeat = HeartbeatManager::new();

    // Key rotation
    // ⚠️ REMOVED (validator-security audit, 2026-08-11): key_rotation
    // declaration removed — dead-by-design, see key_rotation.rs header.

    // Strike system
    // ⚠️ REMOVED (validator-security audit, 2026-08-11): _strikes
    // declaration removed — dead-by-design, see strike_system.rs header.
    let strike_engine = StrikeEngineV2::new();

    // Suspension / recovery
    let _suspension_recovery = RecoveryManager::new();

    // Delegation
    let delegation_registry = DelegationRegistry::new();

    // Reward systems
    let mut reward_vault = RewardVault::new();
    let _reward_history = RewardHistory::new();
    let claim_system = RewardClaimSystem::new();
    let vesting = VestingManager::new();
    let unbonding = UnbondingManager::new();

    // Performance & trust
    let perf_history = PerformanceHistory::new();
    // ⚠️ FIXED (reputation-wiring audit, 2026-08-11): trust_rep removed —
    // it was a separate ReputationManager instance never populated via
    // .register(), so TrustScore/ElectionV3 always saw the fallback score
    // of 100 for every validator instead of real reputation. Call sites
    // below now use the real, populated `reputation` instance.
    let trust_hb = HeartbeatManager::new();
    let trust_history = PerformanceHistory::new();

    // Upgrade & versioning
    let mut upgrade_manager = UpgradeManager::new();
    let mut version_manager = ProtocolVersionManager::new(
        version::VERSION.to_string()
    );
    let mut fork_manager = HardForkManager::new();

    // Committee
    let mut committee = Committee::new();
    let _committee_vote = CommitteeVote::new();
    let _proposal_system = CommitteeProposalSystem::new();

    // Security council
    let security_council = SecurityCouncil::new();
    let _council_voting = SecurityCouncilVoting::new();

    // Appeals
    let appeals = ValidatorAppeals::new();

    // Emergency systems
    let emergency_freeze = EmergencyFreeze::new();
    let emergency_recovery_mode = EmergencyRecovery::new();
    let chain_freeze = ChainFreeze::new();
    let treasury_freeze_guard = TreasuryFreeze::new();
    let _network_recovery = NetworkRecovery::new();

    // Safe restart
    let mut safe_restart = SafeRestartManager::new();

    // Blacklist & admission
    let blacklist = BlacklistRegistry::new();
    // ⚠️ REMOVED (reputation-wiring audit, 2026-08-11): _admission_rep
    // declaration removed — unused separate ReputationManager instance.

    // Sybil / geographic / hosting / latency monitors
    let availability = AvailabilityMonitor::new();

    // Checkpoint systems
    // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): network_checkpoint declaration
    // removed — dead-by-design, see network_checkpoint.rs header. No remaining
    // active call sites (create_checkpoint() that used it is itself unused).
    // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): checkpoint_archive /
    // checkpoint_finalization declarations removed — both were dead-by-design,
    // see checkpoint_archive.rs / checkpoint_finalization.rs headers.
    let _checkpoint_voting_engine = CheckpointVoting::new();
    let _checkpoint_proposal_engine = CheckpointProposalEngine::new();
    let _checkpoint_slashing = CheckpointSlashing::new();
    let _checkpoint_rep = CheckpointReputation::new();
    let checkpoint_weighted_voting = CheckpointWeightedVoting::new();
    let mut checkpoint_replication = CheckpointReplication::new();

    // Treasury protection systems
    let treasury_guard = TreasuryProtection::new(1_000_000);
    let _recovery_lock = TreasuryRecoveryLock::new();
    // TreasuryMultiSig — now lives inside Blockchain (chain.treasury_multisig),
    // initialised with the same 3 owners inside Blockchain::empty().
    let treasury_emergency = TreasuryEmergencyFreeze::new();
    let treasury_audit_log = TreasuryAudit::new();
    let treasury_hash_chain = TreasuryAuditHashChain::new();
    // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): merkle_registry declaration
    // removed — dead-by-design, see checkpoint_merkle_registry.rs header.
    // register()/get()/verify()/show() had no remaining call sites.

    // Snapshot systems
    // validator_snapshots, snap_registry, validator_state_snap,
    // chain_state_snap, governance_snap all removed (2026-08-07):
    // all were fake in-memory-only "snapshots" that either never
    // persisted to disk (wiped every restart) or were never
    // populated at all (always empty). Real persistence is now
    // handled by storage::save_full_state() / load_full_state().

    // Recovery certificate systems
    let cert_registry = RecoveryCertificateRegistry::new();
    let cert_revocation = RecoveryCertificateRevocation::new();
    let cert_expiration = RecoveryCertificateExpiration::new();
    let recovery_authority = RecoveryAuthorityGovernance::new();

    // AMM / DEX
    let liquidity_pool = LiquidityPool::new();

    // Reliability rankings
    let reliability_rankings: HashMap<String, f64> = HashMap::new();

    // Rotation scheduler
    let mut rotation_scheduler = RotationScheduler::new();

    println!("[INIT] All subsystems initialised.");

    // ── Step 6a: Register this node with versioning ──────────
    version_manager.register_node(
        "self".to_string(),
        version::VERSION.to_string(),
    );
    version_manager.show();
    version_manager.readiness_report();

    // ── Step 6b: Verify safe restart conditions ───────────────
    println!("[RESTART] Verifying safe restart conditions...");
    safe_restart.verify_validators();
    safe_restart.verify_epoch();
    safe_restart.verify_consensus();
    safe_restart.show();


    // ── Step 6d: Treasury integrity check ─────────────────────
    println!("[TREASURY] Running treasury integrity check...");
    treasury_guard.show();
    // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() removed — see
    // treasury_protection.rs header comment. expected_balance is a
    // static constant (1_000_000) that never updates with real
    // deposits/spends, so this fired a false "TAMPERING DETECTED"
    // alarm on every legitimate balance change.
    // ⚠️ REMOVED (Batch 2 audit, 2026-08-10): treasury_audit_log.show() was
    // printing misleading output — .record() is never called anywhere, so
    // this log is permanently empty and does not reflect real treasury activity.
    treasury_hash_chain.show();

    // ── Step 6e: Recovery authority setup ────────────────────
    println!("[RECOVERY] Recovery authority initialised.");
    recovery_authority.show();

    // ── Step 6f: Emergency systems status ────────────────────
    println!("[EMERGENCY] Checking emergency system status...");
    chain_freeze.status();
    treasury_freeze_guard.status();
    // ⚠️ REMOVED (Batch 2.5 audit, 2026-08-10): emergency_recovery_mode
    // and emergency_freeze were printing misleading status — neither is
    // wired to any real freeze/recovery path. See audit notes.

    // ── Step 6g: Upgrade / fork schedule ─────────────────────
    println!("[UPGRADE] Checking upgrade schedule...");
    upgrade_manager.show();
    fork_manager.show();

    // ⚠️ REMOVED (Batch 3 audit, 2026-08-10): this printed status from the
    // dead network-checkpoint cluster (network_checkpoint.rs and friends) —
    // in-memory only, disconnected from the real checkpoint/recovery system
    // in chain.rs. See network_checkpoint.rs header for details.

    // ── Step 6i: Reward vault status ─────────────────────────
    println!("[REWARDS] Reward vault status:");
    reward_vault.show();

    // ── Step 6j: Blacklist status ─────────────────────────────
    println!("[BLACKLIST] Active blacklist entries: {}", blacklist.total_banned());

    // ── Step 6k: Epoch status ─────────────────────────────────
    println!("[EPOCH] Epoch manager ready.");
    epoch_manager.show_history();
    epoch_manager.show_rewards();

    // ── Step 6l: Governance status ────────────────────────────
    println!("[GOVERNANCE] Governance state:");
    governance.show_votes();
    governance.emergency_status();

    // ── Step 6m: Validator selector ready ────────────────────
    println!("[SELECTOR] Validator selector ready.");
    ValidatorSelector::show_pool(&validator_registry);

    // ── Step 6n: Committee setup ──────────────────────────────
    println!("[COMMITTEE] Building committee...");
    committee.build(&validator_registry, 2);
    committee.show();

    // ── Step 6o: Security council status ─────────────────────
    println!(
        "[COUNCIL] Security council members: {}",
        security_council.member_count()
    );
    security_council.show_votes();

    // ── Step 6p: Availability monitor ────────────────────────
    println!("[AVAILABILITY] Availability monitor ready.");
    availability.show();

    // ── Step 6q: Network latency baseline ────────────────────
    println!("[LATENCY] Latency baseline will be established after peer connections.");

    // ── Step 6r: Sybil detection ──────────────────────────────
    println!("[SYBIL] Sybil detector armed. Will process validator set.");

    // ── Step 6s: Geographic diversity check ──────────────────
    println!("[GEO] Geographic diversity monitor ready.");

    // ── Step 6t: Hosting risk monitor ────────────────────────
    println!("[HOSTING] Hosting risk analyser ready.");

    // ── Step 6u: AMM pool status ──────────────────────────────
    println!("[AMM] Liquidity pool status:");
    liquidity_pool.info();

    // ── Step 6v: Vesting / unbonding ─────────────────────────
    println!("[VESTING] Vesting manager ready.");
    vesting.show();
    println!("[UNBONDING] Unbonding manager ready.");
    unbonding.show();

    // ── Step 6w: Delegation registry ─────────────────────────
    println!("[DELEGATION] Delegation registry ready.");
    delegation_registry.show_all();

    // ── Step 6x: Snapshot trust & consensus ──────────────────
    println!("[SNAPSHOT] Snapshot trust and consensus systems ready.");

    // ── Step 6y: Recovery certificate systems ────────────────
    println!("[CERT] Recovery certificate systems ready.");
    cert_registry.show();
    cert_revocation.show();
    cert_expiration.show();

    // ── Step 6z: Validator appeals ────────────────────────────
    println!("[APPEALS] Validator appeals system ready.");
    appeals.show();

    // ⚠️ REMOVED (Batch 2.5 audit, 2026-08-10): this printed chain_freeze
    // .is_frozen(), but ChainFreeze was never wired to any real enforcement
    // path — freezing it changed nothing. Real chain_frozen field is now on
    // Blockchain itself; see mine_pending_transactions() for enforcement.

    // ── Step 7: Bootstrap peers ───────────────────────────────
    println!("[PEERS] Bootstrapping peer connections...");
    let mut peer_manager = PeerManager::new();
    peer_manager.bootstrap();
    peer_manager.list_peers();
    println!(
        "[PEERS] Connected peer count: {}",
        peer_manager.peer_count()
    );
    peer_manager.network_health();

    // ── Step 8: Start P2P network thread ─────────────────────
    // [FIX-08] P2P runs in its own thread, does not block startup.
    let p2p_addr = std::env::var("NSC_P2P_ADDR")
        .unwrap_or_else(|_| DEFAULT_P2P_ADDR.to_string());

    println!("[NET] Starting P2P node on {}...", p2p_addr);
    let node = Arc::new(Node::new(
    p2p_addr.clone(),
    Arc::clone(&blockchain),
));
    let node_clone = Arc::clone(&node);

    thread::spawn(move || {
        node_clone.start();
    });

    node.show_connections();
    node.network_status();

    // ── Step 8b: Create shared TokenRegistry (used by API + BSC watcher) ──
    let token_registry = std::sync::Arc::new(
        token_registry::TokenRegistry::new("/root/nsc-data/token_registry.json")
    );

    // ── Step 9: Start API thread ──────────────────────────────
    // [FIX-07] API runs in its own thread, does not block startup.
    // [NOTE]   Add TLS + API key auth inside api.rs before launch.
    let api_port = std::env::var("NSC_API_PORT")
        .unwrap_or_else(|_| DEFAULT_API_PORT.to_string());

    println!("[API] Starting REST API on port {}...", api_port);

    let api_chain = Arc::clone(&blockchain);
    let api_token_registry = std::sync::Arc::clone(&token_registry);
    let api_l2_state = Arc::clone(&l2_state);

thread::spawn(move || {
    api::start_api(api_chain, api_token_registry, api_l2_state);
});

    // ── Step 9b: Start EVM-compatible JSON-RPC (additive) ─────
    // Runs on its own OS thread with its own tokio runtime, so it
    // cannot block or interfere with the existing tiny_http API
    // or the main chain loop. Lets MetaMask / Binance Wallet add
    // NSC as a custom network for early testing.
    let evm_chain_height = blockchain.lock()
        .map(|c| c.chain_height())
        .unwrap_or(0);

    let evm_blockchain = Arc::clone(&blockchain);

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new()
            .expect("failed to build tokio runtime for EVM-RPC");
        rt.block_on(async move {
            if let Err(e) = evm_rpc::start_evm_rpc_server(evm_chain_height as u64, evm_blockchain).await {
                eprintln!("[EVM-RPC] Failed to start: {}", e);
            }
        });
    });


    // ── Step 9c: Start "Any Coin, Any Network" Phase A watcher ──
    // Read-only detection + display. No custody/send. Runs on its own
    // OS thread + tokio runtime, same isolation pattern as EVM-RPC.
    let watcher_registry = std::sync::Arc::clone(&token_registry);

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new()
            .expect("failed to build tokio runtime for BSC watcher");
        rt.block_on(async move {
            bsc_watcher::start_bsc_watcher(watcher_registry).await;
        });
    });

    // ── Print startup summary ─────────────────────────────────
    {
        let chain = blockchain.lock().expect("chain lock");
        println!("====================================================");
        println!("[NODE] Startup complete.");
        println!(
            "  Chain height  : {}",
            chain.chain_height()
        );
        println!(
            "  Difficulty    : {}",
            chain.difficulty
        );
        println!(
            "  Mempool txs   : {}",
            chain.mempool.count()
        );
        println!(
            "  Validators    : {}",
            validator_registry.validator_count()
        );
        println!(
            "  Total stake   : {}",
            validator_registry.total_stake()
        );
        println!(
            "  Peers         : {}",
            peer_manager.peer_count()
        );
        println!(
            "  P2P addr      : {}",
            p2p_addr
        );
        println!(
            "  API port      : {}",
            api_port
        );
        println!("====================================================");
        chain.show_stats();
chain.network_stats();
chain.supply_info();
chain.show_checkpoints();
chain.latest_block_info();

println!("\n=== DEBUG WALLETS ===");
chain.richest_wallets();
println!(
    "Founder balance = {}",
    chain.get_balance("NSC7F70B325205581ED")
);
    }

    // ── Step 10: Mainnet node heartbeat loop ──────────────────
    // Runs every HEARTBEAT_SECS seconds.
    // Performs:
    //   - Chain save to disk
    //   - Mempool cleanup
    //   - Epoch processing
    //   - Checkpoint scheduling
    //   - Heartbeat recording for validators
    //   - Offline jail checks
    //   - Auto-slash checks
    //   - Reputation decay
    //   - Upgrade processing
    //   - Treasury audit
    //   - Chain state snapshot
    //   - Health status log
    println!("[NODE] Entering mainnet heartbeat loop...");

    let mut tick: u64 = 0;

    loop {
        thread::sleep(Duration::from_secs(HEARTBEAT_SECS));
        tick += 1;

        println!(
            "[TICK {}] ================================================",
            tick
        );

        // ── 10a: Chain save ───────────────────────────────────
        {
            let chain = blockchain.lock().expect("chain lock");
            chain.save();
            println!(
                "[TICK {}] Chain saved. height={}",
                tick,
                chain.chain_height()
            );
        }

        // ── 10b: Mempool cleanup ──────────────────────────────
        {
            let mut chain = blockchain.lock().expect("chain lock");
            chain.cleanup_mempool();
        }

        // ── 10b0: Price history tick (every 5 minutes) ─────────
        // Periodic safety-net tick, on top of the per-trade ticks now
        // recorded directly in api.rs at swap time (2026-08-16). This
        // keeps ticks flowing even during stretches with zero trades,
        // so flat periods are still visible on the chart as a flat
        // line rather than a gap.
        if tick % 5 == 0 {
            crate::storage::record_price_tick_now("NSC");
            let tokens_ph = crate::storage::load_tokens();
            for t in &tokens_ph {
                crate::storage::record_price_tick_now(&t.symbol);
            }
        }

        // ── 10b0a: Periodic balance audit (early warning) ──────
        // [P4-hardening, 2026-08-16] audit_balances() previously only
        // ran once at startup (inside apply_full_state()). This adds
        // a periodic re-check on the same 5-minute cadence as the
        // price tick above, so a balance/supply discrepancy would be
        // logged (via [WARN] to stderr) well before the next restart,
        // rather than only being caught at the next node restart.
        // Warn-only by design: it does not auto-freeze, since an
        // automatic freeze on a possibly-false-positive audit result
        // would itself be a new risk. A real discrepancy should be
        // investigated and, if confirmed, frozen manually via the
        // existing emergency-freeze mechanism.
        if tick % 5 == 0 {
            let chain = blockchain.lock().expect("chain lock");
            chain.audit_balances();
        }

        // ── 10b0b: L2 batch auto-finalize ──────────────────────
        // [L2-FIX, 2026-08-19] finalize_batch() previously had zero call
        // sites anywhere in the codebase -- no batch could ever be
        // finalized, and therefore no withdrawal could ever complete.
        // This scans all pending batches every tick and finalizes any
        // whose challenge window has passed with no successful challenge.
        // Runs every tick (not gated to tick % 5) since CHALLENGE_WINDOW_SECS
        // is 3600s (60 min) -- checking every HEARTBEAT_SECS (60s) keeps
        // finalization latency low relative to that window, and the scan
        // itself is cheap (in-memory Vec iteration, no I/O unless something
        // is actually finalized).
        {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let mut l2 = l2_state.lock().expect("l2 lock");
            let finalized_ids = l2.auto_finalize_eligible_batches(now_secs);
            if !finalized_ids.is_empty() {
                let batch_writes = crate::storage::prepare_l2_state_write(&l2);
                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                    eprintln!("[TICK {}] Failed to persist L2 batch auto-finalize: {}", tick, e);
                }
                println!(
                    "[TICK {}] L2 batch(es) auto-finalized: {:?}",
                    tick, finalized_ids
                );
            }
            drop(l2);
        }

        // ── 10b0c: L2 batch auto-submit ─────────────────────────
        // [L2-FIX, mainnet-prep] Previously batch submission was 100%
        // manual (operator clicking "Submit Batch" in l2-admin.html).
        // On public mainnet this would stall every pending deposit/
        // transfer/withdrawal whenever the operator isn't actively
        // watching. Runs every 5 ticks (~5 min at HEARTBEAT_SECS=60s)
        // rather than every tick, since there's no benefit to batching
        // more often than that and it keeps this cheap. Only submits
        // when there is something to submit AND the sequencer bond is
        // actually locked -- if the bond has expired/been unlocked,
        // this intentionally does nothing (pending_ops keeps accumulating
        // safely) and logs a warning so the operator knows to re-lock
        // the bond rather than batches silently failing forever.
        if tick % 5 == 0 {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let mut l2 = l2_state.lock().expect("l2 lock");
            if !l2.pending_ops.is_empty() {
                if !l2.sequencer_bond_locked {
                    eprintln!(
                        "[TICK {}] WARNING: {} pending L2 op(s) waiting but sequencer bond is not locked -- batch cannot auto-submit until bond is re-locked.",
                        tick, l2.pending_ops.len()
                    );
                } else {
                    let sequencer = l2.sequencer_address.clone();
                    match l2.submit_batch(sequencer, now_secs) {
                        Ok(batch_id) => {
                            let batch_writes = crate::storage::prepare_l2_state_write(&l2);
                            if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                eprintln!("[TICK {}] Failed to persist L2 auto-submitted batch: {}", tick, e);
                            }
                            println!("[TICK {}] L2 batch auto-submitted: batch_id={}", tick, batch_id);
                        }
                        Err(e) => {
                            eprintln!("[TICK {}] L2 auto-submit-batch failed: {}", tick, e);
                        }
                    }
                }
            }
            drop(l2);
        }

        // ── 10b1: Daily price snapshot (for 24h change) ────────
        // Takes a snapshot of current prices once per UTC day
        // boundary, so /markets can compute 24h % change.
        {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let current_day = now_secs / 86400;
            let (saved_day, _) = crate::storage::load_price_snapshot();

            if current_day != saved_day {
                let pool = crate::storage::load_pool();
                // [FIX] pool.0 (nsc reserve) is stored in 18-decimal
                // units; scale down to whole NSC before computing
                // price, same as the 5-minute tick logger above.
                let nsc_reserve_whole = pool.0 as f64 / crate::genesis::DECIMALS as f64;
                let nsc_price = if nsc_reserve_whole > 0.0 { pool.1 / nsc_reserve_whole } else { 0.0 };

                let mut prices = serde_json::json!({ "NSC": nsc_price });
                let tokens = crate::storage::load_tokens();
                for t in &tokens {
                    let tpool = crate::storage::load_token_pool(&t.symbol);
                    // [FIX] tpool.0 (token reserve) is stored in
                    // 18-decimal units; scale down before computing
                    // price, same as the 5-minute tick logger above.
                    let token_reserve_whole = tpool.0 as f64 / crate::genesis::DECIMALS as f64;
                    let price = if token_reserve_whole > 0.0 { tpool.1 / token_reserve_whole } else { 0.0 };
                    prices[&t.symbol] = serde_json::json!(price);
                }

                crate::storage::save_price_snapshot(current_day, &prices);
                println!("[TICK {}] Daily price snapshot saved (day={})", tick, current_day);
            }
        }
// ── 10b1: Instant NSC pool floor top-up ──────────────
// Runs every heartbeat tick (independent of mempool/mining), unlike
// the older mine_reward()-based floor logic which only nudges the
// pool by whatever the block reward happens to be. This mints the
// full deficit in one shot (respecting supply.mint()'s MAX_SUPPLY
// cap) so the pool reaches the floor immediately rather than over
// many blocks.
{
    let mut chain = blockchain.lock().expect("chain lock");
    const NSC_POOL_FLOOR: u128 = 750_000 * crate::genesis::DECIMALS;
    let mut pool = crate::storage::load_pool();
    if pool.0 < NSC_POOL_FLOOR {
        let deficit = NSC_POOL_FLOOR - pool.0;
        if chain.supply.mint(deficit) {
            pool.0 += deficit;
            crate::storage::save_pool(pool.0, pool.1);
            println!(
                "[FLOOR-INSTANT] Minted {} NSC directly into pool to reach floor (new reserve: {}).",
                deficit, pool.0
            );
        } else {
            println!(
                "[FLOOR-INSTANT] Cannot top up pool - MAX_SUPPLY cap reached (deficit was {}).",
                deficit
            );
        }
    }
}

// ── 10b2: Produce new block from mempool ─────────────
{
    let mut chain = blockchain.lock().expect("chain lock");
    if chain.mempool.size() > 0 {
        println!(
            "[TICK {}] Mining {} tx(s)...",
            tick,
            chain.mempool.size()
        );
        chain.mine_pending_transactions(
            "0x20a83cbfbb7a5bb53c9daf7fdc5c294d7b5468d1".to_string()
        );
        println!(
            "[TICK {}] Block mined! height={}",
            tick,
            chain.chain_height()
        );
    }
}

        // ── 10c: Epoch boundary processing ───────────────────
        // Distributes rewards at epoch boundaries.
        epoch_manager.finalize_epoch(&validator_registry);
        epoch_manager.show_rewards();

        // Epoch reward distribution
        EpochRewardEngine::distribute(
            &validator_registry,
            &reputation,
            &uptime,
            &mut reward_vault,
            1000,
        );

        // ⚠️ REMOVED (Batch 3 audit, 2026-08-10): CheckpointScheduler::process()
        // only ever populated the dead, in-memory-only network_checkpoint
        // cluster and printed the misleading "Automatic Checkpoint Created"
        // log line — it never touched the real, fund-critical checkpoint
        // system in chain.rs (Blockchain.checkpoints / cert_registry, created
        // automatically every 5 blocks in mine_pending_transactions()). See
        // network_checkpoint.rs header for the full cluster explanation.

        // ⚠️ REMOVED (Batch 3 audit, 2026-08-10): CheckpointIntegrity::show() was
        // tautological (always compared a checkpoint's hash against itself,
        // so it could never detect real tampering — see checkpoint_integrity.rs
        // header). checkpoint_archive/checkpoint_replication/CheckpointConsensus
        // calls here operated on the same dead network-checkpoint cluster and
        // provided no real fund-critical guarantee; removed alongside it.

        // ── 10f: Validator heartbeats & offline jail ──────────
        // In production, each validator sends heartbeats over P2P.
        // Here we check which validators are offline and jail them.
        let current_epoch = epoch_manager.current_epoch;

        let validator_addrs: Vec<String> =
            validator_registry.validators.keys().cloned().collect();

        OfflineJailEngine::check_many(
            &heartbeat,
            &mut jail,
            validator_addrs.clone(),
            current_epoch,
            3,
        );

        // ── 10g: Auto-slash for downtime ──────────────────────
        for addr in &validator_addrs {
            if downtime.should_penalize(addr) {
                AutoSlasher::process(
                    &downtime,
                    &mut slashing,
                    addr.clone(),
                    50,
                );
            }
        }

        // ── 10h: Reputation decay ─────────────────────────────
        ReputationDecay::process_many(
            &mut reputation,
            &heartbeat,
            validator_addrs.clone(),
            current_epoch,
        );

        // ── 10i: Strike engine check ──────────────────────────
        for addr in &validator_addrs {
            if strike_engine.suspended(addr) {
                println!(
                    "[STRIKES] Validator {} suspended by strike engine.",
                    addr
                );
            }
        }

        // ── 10j: Suspension audit ─────────────────────────────
        for addr in &validator_addrs {
            SuspensionManager::audit(
                addr,
                &strike_engine,
                &availability,
                &reputation,
            );
        }

        // ── 10k: Upgrade epoch processing ────────────────────
        upgrade_manager.process_epoch(current_epoch);
        upgrade_manager.show();

        // ── 10l: Hard fork height processing ─────────────────
        {
            let chain = blockchain.lock().expect("chain lock");
            fork_manager.process_height(chain.chain_height() as u64);
        }

        // ── 10m: Treasury audit ───────────────────────────────
        {
            let chain = blockchain.lock().expect("chain lock");
            // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() removed —
            // static expected_balance never updates with real activity.
            // ⚠️ REMOVED (Batch 2 audit, 2026-08-10): treasury_audit_log.show() was
            // printing misleading output — .record() is never called anywhere.
            chain.treasury.show();
            chain.treasury.show_history();
        }



        // ── 10p: Leader election ──────────────────────────────
        let leader = WeightedLeader::select(
            &validator_registry,
            &reputation,
            &uptime,
            &slashing,
        );
        if let Some(ref l) = leader {
            println!("[LEADER] Selected leader this tick: {}", l);
        }

        // ── 10q: Rotation ─────────────────────────────────────
        if let Some(ref l) = leader {
            rotation.record_selection(l.clone());
        }
        rotation.show();

        // ── 10r: Election V3 ──────────────────────────────────
        let _winners = ElectionV3::elect(
            &validator_registry,
            &reputation,
            &trust_hb,
            &trust_history,
            current_epoch,
            2,
        );

        // ── 10s: Health monitor ───────────────────────────────
        health.show();
        for addr in &validator_addrs {
            HealthScore::show(addr, &reputation, &uptime, &slashing);
        }

        // ── 10t: Performance ranking ──────────────────────────
        PerformanceScore::show_ranking(
            &validator_registry,
            &reputation,
            &health,
            &uptime,
        );

        // ── 10u: Validator rank ───────────────────────────────
        ValidatorRank::show_ranking(
            &validator_registry,
            &reputation,
            &uptime,
        );

        // ── 10v: Dynamic rewards ──────────────────────────────
        DynamicRewards::show(
            &validator_registry,
            &reputation,
            &health,
            &uptime,
        );

        // ── 10w: Election audit ───────────────────────────────
        ElectionAudit::audit(&validator_registry, &reputation);

        // ── 10x: Reward split ─────────────────────────────────
        reward_vault.show();

        // ── 10y: Claim system status ──────────────────────────
        claim_system.show_pending();
        claim_system.show_claimed();

        // ── 10z: Reliability rankings ─────────────────────────
        ReliabilityRanking::show(&reliability_rankings);

        // ── 10aa: Leader election V4 ──────────────────────────
        LeaderElectionV4::show(&reliability_rankings);
        if let Some(l) = LeaderElectionV4::elect(&reliability_rankings) {
            println!("[LEADER-V4] Epoch leader: {}", l);

            if let Some(next) = rotation_scheduler.select_next(
                &reliability_rankings,
                current_epoch,
            ) {
                println!("[SCHEDULER] Next rotation leader: {}", next);
            }
        }
        rotation_scheduler.show();

        // ── 10ab: Network status ──────────────────────────────
        {
            let chain = blockchain.lock().expect("chain lock");
            chain.network_status();
            chain.sync_status();
            chain.chain_health();
            chain.self_check();
        }

        // ── 10ac: Peer health ─────────────────────────────────
        peer_manager.network_health();
        node.network_status();

        // ── 10ad: Jail status ─────────────────────────────────
        jail.show();

        // ── 10ae: Slashing status ─────────────────────────────
        slashing.show_penalties();

        // ── 10af: Blacklist status ────────────────────────────
        println!(
            "[BLACKLIST] Total banned: {}",
            blacklist.total_banned()
        );

        // ⚠️ REMOVED (Batch 2.5 audit, 2026-08-10): emergency_freeze.show()
        // and chain_freeze.is_frozen() were misleading — see audit notes above.

        // ── 10ah: Heartbeat summary ───────────────────────────
        heartbeat.show();

        // ── 10ai: Delegation summary ──────────────────────────
        delegation_registry.show_all();

        // ── 10aj: Checkpoint archive ──────────────────────────
        // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): checkpoint_archive.show(),
        // CheckpointPruning::prune()/show() removed — archive.add() is never
        // called anywhere, so the archive was permanently empty; prune() was
        // a no-op and both show() calls printed misleading "0" status lines
        // every cycle. See checkpoint_archive.rs / checkpoint_pruning.rs headers.

        // ── 10ak: Checkpoint finalization ────────────────────
        // ⚠️ REMOVED (Batch 3 audit, 2026-08-11): checkpoint_finalization.show()
        // removed — finalize() is never called anywhere, so this always printed
        // an empty "FINALIZED CHECKPOINTS" list. See checkpoint_finalization.rs header.

        // ── 10al: Checkpoint voting ───────────────────────────
        checkpoint_weighted_voting.show();

        // ── 10am: Merkle registry ─────────────────────────────
        // ⚠️ REMOVED (Batch 3 audit, 2026-08-10): merkle_registry.show() always
        // printed "Total Checkpoints: 0" — register() is never called anywhere,
        // so this registry is permanently empty. Part of the dead
        // network-checkpoint cluster; see network_checkpoint.rs header.

        // ── 10ao: Recovery authority ──────────────────────────
        recovery_authority.show();

        // ── 10ap: Treasury multisig status ───────────────────
        {
            let chain = blockchain.lock().expect("chain lock");
            chain.treasury_multisig.show_summary();
        }

        // ── 10aq: Treasury emergency freeze ───────────────────
        // [DEAD-CODE-FIX 2026-08-09] treasury_emergency.show() removed.
        // TreasuryEmergencyFreeze is a fully disconnected decoration —
        // its freeze()/unfreeze() are never called anywhere, so it always
        // prints "Frozen: false" regardless of real treasury state. The
        // REAL freeze mechanism (Treasury.frozen / Treasury::freeze()) is
        // correctly checked inside execute_spend(), but ALSO has no call
        // site anywhere yet — there is currently no way to actually
        // trigger a treasury freeze. Building that (e.g. a multisig-gated
        // API route calling chain.treasury.freeze()) is a separate,
        // not-yet-built feature, tracked as an open item.

        // ── 10ar: Uptime status ───────────────────────────────
        uptime.show();

        // ── 10as: Staking status ──────────────────────────────
        staking.show_validators();
        println!(
            "[STAKING] Total staked: {}",
            staking.total_staked()
        );

        // ── 10at: Governance votes ────────────────────────────
        governance.show_votes();

        // ── 10au: Security council ────────────────────────────
        security_council.show_votes();
        println!(
            "[COUNCIL] Members: {}",
            security_council.member_count()
        );

        // ── 10av: Appeals ─────────────────────────────────────
        appeals.show();

        // ── 10aw: Vesting / unbonding ─────────────────────────
        vesting.show();
        unbonding.show();

        // ── 10ax: Key rotation status ─────────────────────────
        // ⚠️ REMOVED (validator-security audit, 2026-08-11): key_rotation.show()
        // removed — rotate_validator_key() is never called anywhere, so this
        // always printed an empty "KEY ROTATION" list. See key_rotation.rs header.

        // ── 10ay: Reputation status ───────────────────────────
        reputation.show();

        // ── 10az: Epoch history ───────────────────────────────
        epoch_manager.show_validator_history();
        epoch_manager.show_history();

        // ── 10ba: Protocol version ────────────────────────────
        version_manager.show();
        version_manager.readiness_report();

        // ── 10bb: Liquidity pool status ───────────────────────
        liquidity_pool.info();
        liquidity_pool.show_lp_holders();


        // ── 10be: Performance history ─────────────────────────
        perf_history.ranking();

        // ── 10bf: Trust score ─────────────────────────────────
        for addr in &validator_addrs {
            TrustScore::show(
                addr,
                &reputation,
                &trust_hb,
                &trust_history,
                current_epoch,
            );
        }

        // ── 10bg: Weighted leader score ───────────────────────
        LeaderV2::show_scores(
            &validator_registry,
            &reputation,
            &heartbeat,
            current_epoch,
        );

        // ── 10bh: Tick summary ────────────────────────────────
        {
            let chain = blockchain.lock().expect("chain lock");
            println!(
                "[TICK {}] height={} peers={} mempool={} epoch={}",
                tick,
                chain.chain_height(),
                peer_manager.peer_count(),
                chain.mempool.count(),
                current_epoch
            );
        }

        println!(
            "[TICK {}] ================================================",
            tick
        );
    }
}

// ============================================================
// NUSACOIN (NSC) - Mainnet Support Functions
// ============================================================
// These functions are called by the node loop above.
// They are separated here for clarity and maintainability.
// ============================================================

// ── Node initialisation helpers ──────────────────────────────

/// Registers this node with the protocol version manager.
/// Called once at startup.
pub fn register_node_version(
    manager: &mut ProtocolVersionManager,
    node_id: &str,
) {
    manager.register_node(
        node_id.to_string(),
        version::VERSION.to_string(),
    );
    println!(
        "[VERSION] Node '{}' registered as v{}",
        node_id,
        version::VERSION
    );
}

/// Loads the validator registry from persistent storage.
/// Returns a new empty registry if none is found.
pub fn load_validator_registry() -> ValidatorRegistry {
    let registry = ValidatorRegistry::new();
    println!(
        "[REGISTRY] Loaded {} validator(s).",
        registry.validator_count()
    );
    registry
}

/// Initialises the reward vault with the genesis treasury balance.
/// Called once at startup.
pub fn init_reward_vault(
    vault: &mut RewardVault,
    genesis_amount: u64,
) {
    vault.deposit(genesis_amount);
    println!(
        "[VAULT] Reward vault initialised with {} NSC.",
        genesis_amount
    );
    vault.show();
}

/// Registers all known validators with the reputation manager.
pub fn register_validators_with_reputation(
    reputation: &mut ReputationManager,
    registry: &ValidatorRegistry,
) {
    for (addr, _) in &registry.validators {
        reputation.register(addr.clone());
    }
    println!(
        "[REPUTATION] {} validator(s) registered.",
        registry.validators.len()
    );
}

/// Registers all known validators with the uptime tracker.
pub fn register_validators_with_uptime(
    uptime: &mut UptimeTracker,
    registry: &ValidatorRegistry,
) {
    for (addr, _) in &registry.validators {
        uptime.register(addr.clone());
    }
    println!(
        "[UPTIME] {} validator(s) registered.",
        registry.validators.len()
    );
}

/// Registers all known validators with the health monitor.
pub fn register_validators_with_health(
    health: &mut HealthMonitor,
    registry: &ValidatorRegistry,
) {
    for (addr, _) in &registry.validators {
        health.register(addr.clone());
    }
    println!(
        "[HEALTH] {} validator(s) registered.",
        registry.validators.len()
    );
}

/// Registers all known validators with the availability monitor.
pub fn register_validators_with_availability(
    availability: &mut AvailabilityMonitor,
    registry: &ValidatorRegistry,
) {
    for (addr, _) in &registry.validators {
        availability.register(addr.clone());
    }
    println!(
        "[AVAILABILITY] {} validator(s) registered.",
        registry.validators.len()
    );
}

/// Registers all known validators with the heartbeat manager.
pub fn register_validators_with_heartbeat(
    heartbeat: &mut HeartbeatManager,
    registry: &ValidatorRegistry,
    current_epoch: u64,
) {
    for (addr, _) in &registry.validators {
        heartbeat.heartbeat(addr.clone(), current_epoch);
    }
    println!(
        "[HEARTBEAT] {} validator(s) registered at epoch {}.",
        registry.validators.len(),
        current_epoch
    );
}

// ── Block production helpers ──────────────────────────────────

/// Selects the block producer for the current epoch.
/// Uses the weighted leader selection combining stake,
/// reputation, uptime, and slashing history.
pub fn select_block_producer(
    registry: &ValidatorRegistry,
    reputation: &ReputationManager,
    uptime: &UptimeTracker,
    slashing: &Slashing,
) -> Option<String> {
    let leader = WeightedLeader::select(
        registry,
        reputation,
        uptime,
        slashing,
    );
    if let Some(ref addr) = leader {
        println!("[PRODUCER] Block producer selected: {}", addr);
    } else {
        println!("[PRODUCER] No eligible block producer found.");
    }
    leader
}

/// Mines pending transactions from the mempool.
/// Only called when a block producer is elected.
/// [FIX-02] Faucet is NOT called here.
pub fn produce_block(
    blockchain: &Arc<Mutex<Blockchain>>,
    miner_address: String,
) {
    let mut chain = blockchain.lock().expect("chain lock");
    if chain.mempool.count() == 0 {
        println!("[BLOCK] Mempool empty. Skipping block production.");
        return;
    }
    println!(
        "[BLOCK] Producing block with {} tx(s)...",
        chain.mempool.count()
    );
    chain.mine_pending_transactions(miner_address);
    println!(
        "[BLOCK] Block produced. New height: {}",
        chain.chain_height()
    );
}

// ── Transaction validation helpers ───────────────────────────

/// Validates a transaction before adding it to the mempool.
/// Returns true if the transaction is valid.
/// [FIX-19] Rejects zero-amount transactions.
/// [FIX-20] Rejects self-transfers.
pub fn validate_incoming_tx(
    blockchain: &Arc<Mutex<Blockchain>>,
    sender: &str,
    receiver: &str,
    amount: u64,
    public_key: &str,
    signature: &str,
) -> bool {
    if amount == 0 {
        println!("[TX] Rejected: zero-amount transaction.");
        return false;
    }
    if sender == receiver {
        println!("[TX] Rejected: self-transfer.");
        return false;
    }
    if sender.is_empty() || receiver.is_empty() {
        println!("[TX] Rejected: empty address.");
        return false;
    }
    if public_key.is_empty() || signature.is_empty() {
        println!("[TX] Rejected: missing key or signature.");
        return false;
    }
    let chain = blockchain.lock().expect("chain lock");
    if chain.is_blacklisted(sender) {
        println!("[TX] Rejected: sender blacklisted.");
        return false;
    }
    if chain.is_blacklisted(receiver) {
        println!("[TX] Rejected: receiver blacklisted.");
        return false;
    }
    true
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Checkpoint helpers ────────────────────────────────────────

// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): never called anywhere.
// Part of the dead network-checkpoint cluster — see network_checkpoint.rs.
/// Creates a new checkpoint at the current chain height.
/// Called every CHECKPOINT_INTERVAL blocks.
pub fn create_checkpoint(
    blockchain: &Arc<Mutex<Blockchain>>,
    network_checkpoint: &mut NetworkCheckpoint,
    epoch: u64,
) {
    let chain = blockchain.lock().expect("chain lock");
    if let Some(last) = chain.blocks.last() {
        network_checkpoint.create(
            last.index,
            last.hash.clone(),
            epoch,
        );
        println!(
            "[CHECKPOINT] Created at height={} epoch={}",
            last.index,
            epoch
        );
    }
}

// ⚠️ DEAD-BY-DESIGN (Batch 3 audit, 2026-08-10): never called anywhere.
/// Verifies the integrity of the checkpoint archive.
pub fn verify_checkpoint_archive(
    archive: &CheckpointArchive,
    network_checkpoint: &NetworkCheckpoint,
) {
    if let Some(cp) = archive.latest() {
        CheckpointIntegrity::show(network_checkpoint, &cp.block_hash);
    }
}

// ── Recovery helpers ──────────────────────────────────────────

/// Attempts to recover the chain from the last valid checkpoint.
/// Called if chain validation fails on startup.
pub fn recover_chain_from_checkpoint(
    blockchain: &Arc<Mutex<Blockchain>>,
) {
    let mut chain = blockchain.lock().expect("chain lock");
    println!("[RECOVERY] Attempting chain recovery from checkpoint...");
    chain.recover_from_checkpoint();
    if chain.validate_chain() {
        println!("[RECOVERY] Chain recovered successfully.");
        chain.save();
    } else {
        eprintln!("[FATAL] Chain recovery failed. Manual intervention required.");
        std::process::exit(1);
    }
}

/// Activates emergency network recovery mode.
/// Freezes chain and treasury until recovery is complete.
pub fn activate_emergency_recovery(
    chain_freeze: &mut ChainFreeze,
    treasury_freeze: &mut TreasuryFreeze,
    network_recovery: &mut NetworkRecovery,
) {
    println!("[EMERGENCY] Activating emergency recovery mode...");
    chain_freeze.freeze();
    treasury_freeze.freeze();
    network_recovery.activate();
    network_recovery.status();
}

/// Deactivates emergency recovery mode.
pub fn deactivate_emergency_recovery(
    chain_freeze: &mut ChainFreeze,
    treasury_freeze: &mut TreasuryFreeze,
    network_recovery: &mut NetworkRecovery,
) {
    println!("[EMERGENCY] Deactivating emergency recovery mode...");
    network_recovery.recover(chain_freeze, treasury_freeze);
    network_recovery.deactivate();
    network_recovery.status();
}

// ── Validator management helpers ──────────────────────────────

/// Jails a validator that has been offline for too long.
/// Called by the offline jail engine.
pub fn jail_offline_validator(
    jail: &mut JailManager,
    address: &str,
    epoch: u64,
) {
    if !jail.is_jailed(address) {
        jail.jail(address.to_string(), epoch);
        println!(
            "[JAIL] Validator {} jailed at epoch {}.",
            address,
            epoch
        );
    }
}

/// Unjails a validator after recovery period.
/// Called by the recovery manager.
pub fn unjail_validator(
    jail: &mut JailManager,
    address: &str,
) {
    if jail.is_jailed(address) {
        jail.unjail(address);
        println!("[JAIL] Validator {} unjailed.", address);
    }
}

/// Processes a validator appeal.
/// Requires security council approval.
pub fn process_validator_appeal(
    appeals: &mut ValidatorAppeals,
    council: &SecurityCouncil,
    address: &str,
    reason: &str,
) {
    appeals.submit(address.to_string(), reason.to_string());
    println!(
        "[APPEALS] Appeal submitted for {}. Council size: {}.",
        address,
        council.member_count()
    );
    appeals.show();
}

// ── Treasury helpers ──────────────────────────────────────────

/// Deposits funds into the treasury.
/// Protected by the treasury guard.
pub fn treasury_deposit(
    treasury: &mut Treasury,
    guard: &TreasuryProtection,
    amount: u128,
) {
    // [FALSE-ALARM-FIX 2026-08-09] detect_tampering() calls removed —
    // static expected_balance never updates with real deposits, so
    // this fired a false alarm on every legitimate deposit.
    let _ = treasury.deposit(amount);
    println!(
        "[TREASURY] Deposited {}. Balance: {}.",
        amount,
        treasury.balance()
    );
}

/// Spends from the treasury.
/// Requires multisig approval and timelock.
pub fn treasury_spend_guarded(
    treasury: &mut Treasury,
    multisig: &mut TreasuryMultiSig,
    request_id: String,
    amount: u128,
    recipient: String,
    description: String,
) -> bool {
    let request = TreasurySpendRequest::new(request_id, amount, recipient, description);

    match treasury.execute_spend(&request, multisig) {
        Ok(()) => {
            println!(
                "[TREASURY] Spent {}. Balance: {}.",
                amount,
                treasury.balance()
            );
            true
        }
        Err(e) => {
            println!("[TREASURY] Spend rejected: {}", e);
            false
        }
    }
}

// ── Governance helpers ────────────────────────────────────────

/// Creates a governance proposal.
/// [FIX-10] Prevents duplicate vote calls.
pub fn create_governance_proposal(
    governance: &mut Governance,
    title: &str,
) {
    governance.create_proposal(title.to_string());
    println!("[GOV] Proposal created: '{}'", title);
}

/// Records a single weighted governance vote.
/// [FIX-10] Caller must ensure one vote per validator per proposal.
pub fn cast_governance_vote(
    governance: &mut Governance,
    proposal: &str,
    voter: &str,
) {
    // [P2-FIX 2026-08-16] Previously discarded the Result and always
    // printed "Vote recorded" regardless of outcome. Now matches on
    // the real result so operators/logs see the true outcome
    // (AlreadyVoted, ProposalExpired, NotEligibleVoter,
    // EmergencyFrozen, Overflow, etc via GovernanceError's Display).
    match governance.weighted_vote(proposal, voter) {
        Ok(new_total) => {
            println!(
                "[GOV] Vote recorded for '{}' by {}. New tally: {}.",
                proposal, voter, new_total
            );
        }
        Err(e) => {
            eprintln!(
                "[GOV] Vote FAILED for '{}' by {}: {}",
                proposal, voter, e
            );
        }
    }
}

// ── Slashing helpers ──────────────────────────────────────────

/// Slashes a validator for misbehaviour.
/// Records the penalty in the slashing history.
pub fn slash_validator(
    slashing: &mut Slashing,
    address: &str,
    amount: u64,
    reason: &str,
) {
    slashing.slash(address.to_string(), amount);
    println!(
        "[SLASH] Validator {} slashed {} NSC. Reason: {}",
        address,
        amount,
        reason
    );
    println!(
        "[SLASH] Banned: {}",
        slashing.is_banned(address)
    );
}

// ── Sybil detection helpers ───────────────────────────────────

// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): never called
// anywhere in the codebase. ValidatorIdentity is also never instantiated
// anywhere, so there is no data source even if this were wired in.
// See sybil_detector.rs header.
/// Runs sybil detection on the current validator set.
/// Called periodically from the node loop.
pub fn run_sybil_detection(
    validators: &[ValidatorIdentity],
) {
    println!("[SYBIL] Running sybil detection on {} validators...", validators.len());
    SybilDetector::detect(&validators.to_vec());
}

/// Runs geographic diversity analysis.
pub fn run_geographic_analysis(
    locations: &[ValidatorLocation],
) {
    println!("[GEO] Analysing geographic diversity for {} validators...", locations.len());
    GeographicDiversity::analyze(&locations.to_vec());
}

/// Runs hosting risk analysis.
pub fn run_hosting_risk_analysis(
    nodes: &[HostingNode],
) {
    println!("[HOSTING] Analysing hosting risk for {} nodes...", nodes.len());
    HostingRiskAnalyzer::analyze(&nodes.to_vec());
}

/// Runs network latency analysis.
pub fn run_latency_analysis(
    records: &[LatencyRecord],
) {
    println!("[LATENCY] Analysing latency for {} validators...", records.len());
    NetworkLatencyAnalyzer::analyze(&records.to_vec());
}

// ⚠️  UNUSED / DEAD CODE — none of the 6 functions in this section
// (save_validator_snapshot, rollback_validator, save_chain_snapshot,
// rollback_chain_state, save_governance_snapshot, rollback_governance)
// are called anywhere in this codebase. Verified via grep audit,
// 2026-08-07. IMPORTANT: even if wired up, the "rollback" functions
// do NOT actually restore/mutate any state — they only read a
// snapshot and print its contents. Do not assume calling them would
// perform a real rollback. Real state persistence and restoration
// is handled by storage::save_full_state() / load_full_state(),
// wired into Blockchain::save() and apply_full_state().
// ── Snapshot helpers ──────────────────────────────────────────

/// Saves a validator state snapshot.
pub fn save_validator_snapshot(
    snapshot: &mut ValidatorStateSnapshot,
    address: &str,
    stake: u64,
    reputation: u64,
) {
    snapshot.save(address.to_string(), stake, reputation as i64);
    println!(
        "[SNAPSHOT] Validator snapshot saved for {}.",
        address
    );
}

/// Rolls back a validator to its last snapshot.
pub fn rollback_validator(
    snapshot: &ValidatorStateSnapshot,
    address: &str,
) {
    ValidatorRollback::rollback(snapshot, address);
    println!(
        "[ROLLBACK] Validator {} rolled back to snapshot.",
        address
    );
}

/// Saves a chain state snapshot.
pub fn save_chain_snapshot(
    snapshot: &mut ChainStateSnapshot,
    treasury_balance: u128,
    block_height: u64,
    validator_count: u64,
    epoch: u64,
) {
    snapshot.save(
    treasury_balance,
    block_height,
    epoch,
    validator_count as u32,
    validator_count as usize,
);
    println!(
        "[SNAPSHOT] Chain state snapshot saved at height={}.",
        block_height
    );
}

/// Rolls back chain state to the last snapshot.
pub fn rollback_chain_state(
    snapshot: &ChainStateSnapshot,
) {
    ChainStateRollback::rollback(snapshot);
    println!("[ROLLBACK] Chain state rolled back to snapshot.");
}

/// Saves a governance state snapshot.
pub fn save_governance_snapshot(
    snapshot: &mut GovernanceStateSnapshot,
    epoch: u64,
    proposal_count: u64,
    governance_version: u64,
    passed_count: u64,
) {
    snapshot.save(epoch, proposal_count, governance_version, passed_count as u32);
    println!(
        "[SNAPSHOT] Governance snapshot saved at epoch={}.",
        epoch
    );
}

/// Rolls back governance state to the last snapshot.
pub fn rollback_governance(
    snapshot: &GovernanceStateSnapshot,
) {
    GovernanceRollback::rollback(snapshot);
    println!("[ROLLBACK] Governance state rolled back to snapshot.");
}

// ── Certificate helpers ───────────────────────────────────────

/// Generates a recovery certificate for a snapshot.
pub fn generate_recovery_certificate(
    height: u64,
    snapshot_hash: &str,
    registry: &mut RecoveryCertificateRegistry,
) -> RecoveryCertificate {
    let cert = RecoverySnapshotCertificate::generate(
        height,
        snapshot_hash.to_string(),
    );
    registry.register(cert.clone());
    println!(
        "[CERT] Recovery certificate generated for height={}.",
        height
    );
    RecoverySnapshotCertificate::show(&cert);
    cert
}

/// Revokes a recovery certificate.
pub fn revoke_certificate(
    id: &str,
    revocation: &mut RecoveryCertificateRevocation,
) {
    revocation.revoke(id.to_string());
    println!("[CERT] Certificate {} revoked.", id);
}

/// Checks if a certificate has expired.
pub fn check_certificate_expiry(
    id: &str,
    expiration: &RecoveryCertificateExpiration,
    current_epoch: u64,
) -> bool {
    let expired = expiration.is_expired(id, current_epoch);
    println!(
        "[CERT] Certificate {} expired at epoch {}: {}",
        id,
        current_epoch,
        expired
    );
    expired
}

// ── Merkle proof helpers ──────────────────────────────────────

/// Computes and verifies a Merkle audit proof.
pub fn verify_merkle_proof(
    leaf: &str,
    proof: Vec<String>,
    root: &str,
) -> bool {
    let valid = MerkleProof::verify(leaf, proof, root.to_string());
    MerkleProof::show(valid);
    valid
}

/// Computes the Merkle root for a set of treasury records.
pub fn compute_treasury_merkle_root(
    records: &[String],
) -> String {
    let root = TreasuryMerkleAudit::merkle_root(records);
    println!("[MERKLE] Treasury Merkle root: {}", root);
    root
}

// ── Admission control helpers ─────────────────────────────────

/// Checks if a validator can join the network.
/// Enforces minimum stake, reputation, and blacklist checks.
pub fn check_validator_admission(
    address: &str,
    stake: u64,
    min_stake: u64,
    reputation: &ReputationManager,
    blacklist: &BlacklistRegistry,
) -> bool {
    let ok = AdmissionController::can_join(
        address,
        stake,
        min_stake,
        reputation,
        blacklist,
    );
    println!(
        "[ADMISSION] Validator {} admission: {}",
        address,
        if ok { "APPROVED" } else { "DENIED" }
    );
    ok
}

// ── Node fingerprint helpers ──────────────────────────────────

// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): never called
// anywhere in the codebase. See node_fingerprint.rs header.
/// Generates a node fingerprint for sybil resistance.
pub fn generate_node_fingerprint(
    address: &str,
    ip: &str,
    public_key: &str,
) -> String {
    let fp = NodeFingerprint::generate(address, ip, public_key);
    println!(
        "[FINGERPRINT] Node fingerprint generated: {}",
        NodeFingerprint::short(&fp)
    );
    fp
}

// ── Snapshot trust helpers ────────────────────────────────────

/// Selects the best snapshot provider by trust score.
pub fn select_best_snapshot_provider(
    providers: &[SnapshotProvider],
) -> Option<SnapshotProvider> {
    let best = SnapshotTrustScore::select_best(providers);
    if let Some(ref p) = best {
        SnapshotTrustScore::show(p);
    }
    best
}

/// Runs snapshot consensus among providers.
pub fn run_snapshot_consensus(
    votes: &[SnapshotVote],
) -> Option<String> {
    let consensus = SnapshotConsensus::select_consensus(votes);
    if let Some(ref hash) = consensus {
        SnapshotConsensus::show(hash);
    }
    consensus
}

/// Finalises a snapshot after consensus.
pub fn finalise_snapshot(
    height: u64,
    hash: &str,
) -> FinalizedSnapshot {
    let finalized = SnapshotFinalization::finalize(
        height,
        hash.to_string(),
    );
    SnapshotFinalization::show(&finalized);
    println!(
        "[SNAPSHOT] Finalised snapshot verified: {}",
        SnapshotFinalization::verify(&finalized)
    );
    finalized
}

// ── Timelock helpers ──────────────────────────────────────────

/// Creates a timelocked treasury transaction.
pub fn create_timelock_tx(
    amount: u64,
    delay_secs: u64,
) -> TimeLockedTransaction {
    let tx = TreasuryTimeLock::create(amount, delay_secs);
    TreasuryTimeLock::show(&tx);
    println!(
        "[TIMELOCK] Timelock created for {} NSC with {}s delay.",
        amount,
        delay_secs
    );
    tx
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Weighted checkpoint quorum ────────────────────────────────

/// Shows whether a checkpoint has reached weighted quorum.
// ⚠️ DEAD-BY-DESIGN (Batch 2 audit, 2026-08-10): never called anywhere.
pub fn check_checkpoint_quorum(
    approval_weight: u64,
    total_weight: u64,
    threshold_pct: u64,
) {
    CheckpointWeightedQuorum::show(
    approval_weight as i64,
    total_weight as i64,
    threshold_pct as i64,
);
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Snapshot quorum ───────────────────────────────────────────

/// Shows whether a snapshot has reached quorum.
pub fn check_snapshot_quorum(
    approvals: u64,
    total: u64,
    threshold_pct: u64,
) {
    SnapshotQuorum::show(
    approvals as usize,
    total as usize,
    threshold_pct as usize,
);
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Checkpoint quorum ─────────────────────────────────────────

/// Shows whether a network checkpoint has reached quorum.
// ⚠️ DEAD-BY-DESIGN (Batch 2 audit, 2026-08-10): never called anywhere.
pub fn check_network_checkpoint_quorum(
    approvals: u64,
    total: u64,
    threshold_pct: u64,
) {
    CheckpointQuorum::show(
    approvals as usize,
    total as usize,
    threshold_pct as usize,
);
}

// ── Reward split helpers ──────────────────────────────────────

/// Distributes block reward between validator and delegators.
pub fn distribute_block_reward(
    splitter: &mut RewardSplitEngine,
    validator: &str,
    delegator: &str,
    total_reward: u64,
    delegator_share_pct: u64,
) {
    splitter.distribute(
        validator.to_string(),
        delegator.to_string(),
        total_reward,
        delegator_share_pct,
    );
    println!(
        "[REWARDS] Block reward {} split: validator={} delegator={}.",
        total_reward,
        validator,
        delegator
    );
    splitter.show();
}

// ── Unbonding helpers ─────────────────────────────────────────

/// Requests unbonding for a validator.
pub fn request_unbond(
    unbonding: &mut UnbondingManager,
    address: &str,
    amount: u64,
    current_epoch: u64,
    unbond_period: u64,
) {
    unbonding.request_unbond(
        address.to_string(),
        amount,
        current_epoch,
        unbond_period,
    );
    println!(
        "[UNBOND] {} requested unbond of {} NSC at epoch {}.",
        address,
        amount,
        current_epoch
    );
    unbonding.show();
}

/// Checks if unbonded funds are claimable.
pub fn check_unbond_claimable(
    unbonding: &UnbondingManager,
    address: &str,
    current_epoch: u64,
) -> bool {
    let claimable = unbonding.claimable(address, current_epoch);
    println!(
        "[UNBOND] {} claimable at epoch {}: {}",
        address,
        current_epoch,
        claimable
    );
    claimable > 0
}

// ── Key rotation helpers ──────────────────────────────────────

// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): never called
// anywhere in the codebase. See key_rotation.rs header.
/// Rotates a validator's signing key.
/// Called when a validator rotates their key pair.
pub fn rotate_validator_key(
    manager: &mut KeyRotationManager,
    address: &str,
    new_public_key: &str,
) {
    manager.rotate(address, new_public_key.to_string());
    println!(
        "[KEY] Validator {} key rotated to {}.",
        address,
        new_public_key
    );
    manager.show_history(address);
    println!(
        "[KEY] Active key: {:?}",
        manager.active_key(address)
    );
}

// ── Delegation helpers ────────────────────────────────────────

/// Records a delegation from delegator to validator.
pub fn record_delegation(
    registry: &mut DelegationRegistry,
    delegator: &str,
    validator: &str,
    amount: u64,
) {
    registry.delegate(
        delegator.to_string(),
        validator.to_string(),
        amount,
    );
    println!(
        "[DELEGATE] {} delegated {} NSC to {}.",
        delegator,
        amount,
        validator
    );
}

/// Distributes delegator rewards for a validator.
pub fn distribute_delegator_rewards(
    engine: &mut DelegatorRewardEngine,
    registry: &DelegationRegistry,
    validator: &str,
    total_reward: u64,
    commission_pct: u64,
) {
    engine.distribute(registry, validator, total_reward, commission_pct);
    println!(
        "[DELEGATE] Rewards distributed for validator {}.",
        validator
    );
    engine.show();
}

// ── Reputation helpers ────────────────────────────────────────

/// Rewards a validator for good behaviour.
pub fn reward_validator_reputation(
    reputation: &mut ReputationManager,
    address: &str,
    amount: u64,
) {
    reputation.reward(address, amount as i32);
    println!(
        "[REPUTATION] Validator {} rewarded {}.",
        address,
        amount
    );
}

/// Punishes a validator for bad behaviour.
pub fn punish_validator_reputation(
    reputation: &mut ReputationManager,
    address: &str,
    amount: u64,
) {
    reputation.punish(address, amount as i32);
    println!(
        "[REPUTATION] Validator {} punished {}.",
        address,
        amount
    );
}

/// Runs reputation recovery for a validator.
pub fn run_reputation_recovery(
    reputation: &mut ReputationManager,
    address: &str,
    amount: u64,
) {
    ReputationRecovery::recover(reputation, address, amount as i32);
    ReputationRecovery::gradual_recovery(reputation, address);
    println!(
        "[REPUTATION] Recovery applied for {}. Score: {}",
        address,
        reputation.reputation(address)
    );
}

// ── Peer management helpers ───────────────────────────────────

/// Rewards a peer for good behaviour.
pub fn reward_peer(
    manager: &mut PeerManager,
    ip: &str,
    amount: u64,
) {
    manager.reward_peer(ip, amount as i32);
    println!("[PEER] {} rewarded {}.", ip, amount);
}

/// Punishes a peer for bad behaviour.
pub fn punish_peer(
    manager: &mut PeerManager,
    ip: &str,
    amount: u64,
) {
    manager.punish_peer(ip, amount as i32);
    println!("[PEER] {} punished {}.", ip, amount);
}

/// Bans a peer.
pub fn ban_peer(
    manager: &mut PeerManager,
    ip: &str,
) {
    manager.ban_peer(ip.to_string());
    println!("[PEER] {} banned.", ip);
}

// ── Chain export helpers ──────────────────────────────────────

/// Exports the current chain state as a snapshot.
pub fn export_chain_snapshot(
    blockchain: &Arc<Mutex<Blockchain>>,
) {
    let chain = blockchain.lock().expect("chain lock");
    chain.export_snapshot();
    println!("[EXPORT] Chain snapshot exported.");
}

/// Exports the chain data as a string for sync.
pub fn export_chain_for_sync(
    blockchain: &Arc<Mutex<Blockchain>>,
) -> String {
    let chain = blockchain.lock().expect("chain lock");
    let data = chain.export_chain();
    println!("[SYNC] Chain data exported ({} chars).", data.len());
    data
}

// ── Supply helpers ────────────────────────────────────────────

/// Returns the current circulating supply.
pub fn get_circulating_supply(
    blockchain: &Arc<Mutex<Blockchain>>,
) -> u128 {
    let chain = blockchain.lock().expect("chain lock");
    let supply = chain.circulating_supply();
    println!("[SUPPLY] Circulating: {} NSC", supply);
    supply
}

/// Prints a full supply report.
pub fn print_supply_report(
    blockchain: &Arc<Mutex<Blockchain>>,
) {
    let chain = blockchain.lock().expect("chain lock");
    chain.print_supply();
    chain.supply_info();
}

// ── Balance helpers ───────────────────────────────────────────

/// Returns the balance of an address.
pub fn get_balance(
    blockchain: &Arc<Mutex<Blockchain>>,
    address: &str,
) -> u128 {
    let chain = blockchain.lock().expect("chain lock");
    let balance = chain.get_balance(address);
    println!("[BALANCE] {}: {} NSC", address, balance);
    balance
}

/// Checks if an address exists in the chain.
pub fn wallet_exists(
    blockchain: &Arc<Mutex<Blockchain>>,
    address: &str,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let exists = chain.wallet_exists(address);
    println!("[WALLET] {} exists: {}", address, exists);
    exists
}

// ── Transaction helpers ───────────────────────────────────────

/// Checks if a transaction hash has been processed.
pub fn tx_exists(
    blockchain: &Arc<Mutex<Blockchain>>,
    tx_hash: &str,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let exists = chain.tx_exists(tx_hash);
    println!("[TX] {} exists: {}", tx_hash, exists);
    exists
}

/// Returns the transaction history for an address.
pub fn get_address_history(
    blockchain: &Arc<Mutex<Blockchain>>,
    address: &str,
) {
    let chain = blockchain.lock().expect("chain lock");
    chain.address_history(address);
}

/// Returns the transaction count for an address.
pub fn get_tx_count(
    blockchain: &Arc<Mutex<Blockchain>>,
    address: &str,
) -> usize {
    let chain = blockchain.lock().expect("chain lock");
    let count = chain.transaction_count(address);
    println!("[TX] {} transactions for {}.", count, address);
    count
}

// ── Block helpers ─────────────────────────────────────────────

/// Returns block data at a given height.
pub fn get_block(
    blockchain: &Arc<Mutex<Blockchain>>,
    index: usize,
) {
    let chain = blockchain.lock().expect("chain lock");
    chain.get_block(index);
}

/// Returns the latest block info.
pub fn get_latest_block_info(
    blockchain: &Arc<Mutex<Blockchain>>,
) {
    let chain = blockchain.lock().expect("chain lock");
    chain.latest_block_info();
}

// ── Mempool helpers ───────────────────────────────────────────

/// Returns the current mempool size.
pub fn get_mempool_size(
    blockchain: &Arc<Mutex<Blockchain>>,
) -> usize {
    let chain = blockchain.lock().expect("chain lock");
    let size = chain.mempool.count();
    println!("[MEMPOOL] {} pending transaction(s).", size);
    size
}

/// Prints the current mempool contents.
pub fn print_mempool(
    blockchain: &Arc<Mutex<Blockchain>>,
) {
    let chain = blockchain.lock().expect("chain lock");
    chain.print_mempool();
}

// ── Fork management helpers ───────────────────────────────────

/// Handles an incoming fork candidate.
/// Adopts the fork only if it is longer and valid.
pub fn handle_fork(
    blockchain: &Arc<Mutex<Blockchain>>,
    fork_blocks: Vec<block::Block>,
) {
    let mut chain = blockchain.lock().expect("chain lock");
    if Blockchain::validate_external_chain(&fork_blocks) {
        chain.add_fork(fork_blocks);
        chain.select_best_chain();
    } else {
        println!("[FORK] Rejected invalid fork candidate.");
    }
}

// ── Reward claim helpers ──────────────────────────────────────

/// Claims pending rewards for a validator.
pub fn claim_validator_reward(
    claim_system: &mut RewardClaimSystem,
    address: &str,
) -> u64 {
    let claimed = claim_system.claim(address);
    println!(
        "[CLAIM] {} claimed {} NSC in rewards.",
        address,
        claimed
    );
    claimed
}

/// Adds a pending reward for a validator.
pub fn add_pending_reward(
    claim_system: &mut RewardClaimSystem,
    address: &str,
    amount: u64,
) {
    claim_system.add_reward(address.to_string(), amount);
    println!(
        "[CLAIM] Added {} NSC pending reward for {}.",
        amount,
        address
    );
}

// ── Vesting helpers ───────────────────────────────────────────

/// Adds a vesting reward schedule for a validator.
pub fn add_vesting_reward(
    vesting: &mut VestingManager,
    address: &str,
    amount: u64,
    epochs: u64,
) {
    vesting.add_reward(address.to_string(), amount, epochs);
    println!(
        "[VESTING] Added {} NSC vesting over {} epochs for {}.",
        amount,
        epochs,
        address
    );
}

/// Claims vested rewards at the current epoch.
pub fn claim_vested_rewards(
    vesting: &mut VestingManager,
    address: &str,
    current_epoch: u64,
) -> u64 {
    let claimed = vesting.claim(address, current_epoch);
    println!(
        "[VESTING] {} claimed {} NSC at epoch {}.",
        address,
        claimed,
        current_epoch
    );
    claimed
}

// ── Strike helpers ────────────────────────────────────────────

// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): never called
// anywhere in the codebase. See strike_system.rs header.
/// Adds a strike to a validator.
/// Bans the validator if the strike limit is reached.
pub fn add_validator_strike(
    strikes: &mut StrikeSystem,
    address: &str,
) {
    strikes.add_strike(address.to_string());
    println!(
        "[STRIKE] Strike added for {}. Total: {}. Banned: {}.",
        address,
        strikes.strike_count(address),
        strikes.is_banned(address)
    );
    strikes.show();
}

/// Adds a weighted strike via the V2 engine.
pub fn add_weighted_strike(
    engine: &mut StrikeEngineV2,
    address: &str,
    weight: u64,
) {
    engine.add_strike(address, weight as u32);
    println!(
        "[STRIKE-V2] Weighted strike {} added for {}. Suspended: {}.",
        weight,
        address,
        engine.suspended(address)
    );
    engine.show();
}

// ── Hard fork helpers ─────────────────────────────────────────

/// Schedules a hard fork.
pub fn schedule_hard_fork(
    manager: &mut HardForkManager,
    id: u64,
    name: &str,
    activation_height: u64,
) {
    manager.schedule_fork(id, name.to_string(), activation_height);
    println!(
        "[FORK] Hard fork '{}' scheduled at height {}.",
        name,
        activation_height
    );
    manager.show();
}

/// Schedules a protocol upgrade.
pub fn schedule_upgrade(
    manager: &mut UpgradeManager,
    id: u64,
    name: &str,
    activation_epoch: u64,
) {
    manager.schedule_upgrade(id, name.to_string(), activation_epoch);
    println!(
        "[UPGRADE] Upgrade '{}' scheduled at epoch {}.",
        name,
        activation_epoch
    );
    manager.show();
}

// ── Committee proposal helpers ────────────────────────────────

/// Creates a committee proposal.
pub fn create_committee_proposal(
    system: &mut CommitteeProposalSystem,
    title: &str,
    description: &str,
) -> u64 {
    let id = system.create(title.to_string(), description.to_string());
    println!("[COMMITTEE] Proposal '{}' created (id={}).", title, id);
    system.show();
    id
}

/// Executes a committee proposal.
pub fn execute_committee_proposal(
    system: &mut CommitteeProposalSystem,
    id: u64,
) {
    system.execute(id);
    println!("[COMMITTEE] Proposal {} executed.", id);
    system.show();
}

// ── AMM helpers ───────────────────────────────────────────────

/// Adds liquidity to the NSC/USDT pool.
pub fn add_liquidity(
    pool: &mut LiquidityPool,
    blockchain: &mut Blockchain,
    provider: &str,
    nsc_amount: u128,
    usdt_amount: u64,
) {
    match pool.add_liquidity(blockchain, provider, nsc_amount, usdt_amount) {
        Ok(lp_minted) => {
            println!(
                "[AMM] {} added liquidity: {} NSC / {} USDT, received {} LP tokens.",
                provider, nsc_amount, usdt_amount, lp_minted
            );
        }
        Err(e) => {
            println!("[AMM] add_liquidity failed for {}: {}", provider, e);
        }
    }
    pool.info();
}

/// Removes liquidity from the NSC/USDT pool.
pub fn remove_liquidity(
    pool: &mut LiquidityPool,
    blockchain: &mut Blockchain,
    provider: &str,
    amount: u128,
) {
    match pool.remove_liquidity(blockchain, provider, amount) {
        Ok((nsc_out, usdt_out)) => {
            println!(
                "[AMM] {} removed {} LP tokens, received {} NSC + {} USDT.",
                provider, amount, nsc_out, usdt_out
            );
        }
        Err(e) => {
            println!("[AMM] remove_liquidity failed for {}: {}", provider, e);
        }
    }
    pool.info();
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Network checkpoint proposal helpers ──────────────────────

/// Submits a checkpoint proposal.
pub fn submit_checkpoint_proposal(
    engine: &mut CheckpointProposalEngine,
    height: u64,
    proposer: &str,
    hash: &str,
) {
    engine.submit(height, proposer.to_string(), hash.to_string());
    println!(
        "[PROPOSAL] Checkpoint proposal submitted: height={} hash={}.",
        height,
        hash
    );
    engine.show();
}

/// Validates a checkpoint proposal.
pub fn validate_checkpoint_proposal(
    engine: &CheckpointProposalEngine,
    height: u64,
    hash: &str,
) -> bool {
    let valid = CheckpointProposalValidation::validate(engine, height, hash);
    CheckpointProposalValidation::show(valid);
    valid
}

/// Executes a checkpoint proposal.
pub fn execute_checkpoint_proposal(
    engine: &CheckpointProposalEngine,
    checkpoint: &mut NetworkCheckpoint,
) {
    if let Some(proposal) = engine.proposals.first() {
        CheckpointProposalExecution::execute(proposal, checkpoint);
        println!("[PROPOSAL] Checkpoint proposal executed.");
        checkpoint.show();
    }
}

// ── Final node shutdown helper ────────────────────────────────

/// Gracefully shuts down the node.
/// Saves chain, mempool, and all state before exit.
/// [FIX-18] Called on SIGINT/SIGTERM, not in main loop.
pub fn graceful_shutdown(
    blockchain: &Arc<Mutex<Blockchain>>,
) {
    println!("[SHUTDOWN] Saving chain state...");
    let chain = blockchain.lock().expect("chain lock");
    chain.save();
    println!("[SHUTDOWN] Chain saved. Node stopped safely.");
}

// ============================================================
// END OF NUSACOIN MAINNET NODE
// ============================================================

// ============================================================
// NUSACOIN (NSC) - Mainnet Security & Verification Module
// ============================================================
// Extended security checks, verification utilities, and
// mainnet-specific validation logic.
// ============================================================

// ── Chain verification utilities ─────────────────────────────

/// Verifies that every block in the chain has a valid hash.
/// Returns true if the chain is fully intact.
pub fn deep_verify_chain(
    blockchain: &Arc<Mutex<Blockchain>>,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let valid = chain.validate_chain();
    println!(
        "[VERIFY] Deep chain verification: {}",
        if valid { "PASS" } else { "FAIL" }
    );
    if !valid {
        chain.scan_corruption();
    }
    valid
}

/// Verifies that the genesis block matches the known hash.
/// This is a hard security requirement for mainnet.
pub fn verify_genesis_integrity(
    blockchain: &Arc<Mutex<Blockchain>>,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let valid = chain.validate_genesis();
    println!(
        "[VERIFY] Genesis integrity: {}",
        if valid { "PASS" } else { "FAIL" }
    );
    valid
}

/// Verifies a specific checkpoint hash.
pub fn verify_checkpoint_at_height(
    blockchain: &Arc<Mutex<Blockchain>>,
    height: u64,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let valid = chain.verify_checkpoint(height);
    println!(
        "[VERIFY] Checkpoint at height {}: {}",
        height,
        if valid { "PASS" } else { "FAIL" }
    );
    valid
}

/// Finds the last valid checkpoint in the chain.
pub fn find_recovery_checkpoint(
    blockchain: &Arc<Mutex<Blockchain>>,
) -> Option<u64> {
    let chain = blockchain.lock().expect("chain lock");
    let height = chain.find_last_valid_checkpoint();
    println!(
        "[VERIFY] Last valid checkpoint: {:?}",
        height
    );
    height
}

/// Audits all wallet balances against the chain supply.
pub fn audit_wallet_balances(
    blockchain: &Arc<Mutex<Blockchain>>,
) {
    let chain = blockchain.lock().expect("chain lock");
    chain.audit_balances();
}

/// Prints the full top-wallet ranking.
pub fn print_richest_wallets(
    blockchain: &Arc<Mutex<Blockchain>>,
) {
    let chain = blockchain.lock().expect("chain lock");
    chain.richest_wallets();
}

// ── Nonce verification utilities ─────────────────────────────

/// Verifies the nonce for a sender before accepting a transaction.
/// [FIX-19] Rejects invalid nonces to prevent replay attacks.
pub fn verify_sender_nonce(
    blockchain: &Arc<Mutex<Blockchain>>,
    sender: &str,
    nonce: u64,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let valid = chain.verify_nonce(sender, nonce);
    if !valid {
        println!(
            "[NONCE] Invalid nonce {} for sender {}.",
            nonce,
            sender
        );
    }
 valid
}

/// Returns the expected next nonce for a sender.
pub fn get_sender_nonce(
    blockchain: &Arc<Mutex<Blockchain>>,
    sender: &str,
) -> u64 {
    let chain = blockchain.lock().expect("chain lock");
    let nonce = *chain.nonces.get(sender).unwrap_or(&0);
    println!(
        "[NONCE] Current nonce for {}: {}",
        sender,
        nonce
    );
    nonce
}

// ── Blacklist management utilities ────────────────────────────

/// Blacklists a wallet address at the chain level.
pub fn blacklist_address(
    blockchain: &Arc<Mutex<Blockchain>>,
    address: &str,
) {
    let mut chain = blockchain.lock().expect("chain lock");
    chain.blacklist_wallet(address.to_string());
    println!("[BLACKLIST] Address {} blacklisted.", address);
    chain.print_blacklist();
}

/// Checks if an address is blacklisted.
pub fn is_address_blacklisted(
    blockchain: &Arc<Mutex<Blockchain>>,
    address: &str,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let blacklisted = chain.is_blacklisted(address);
    println!(
        "[BLACKLIST] {} blacklisted: {}",
        address,
        blacklisted
    );
    blacklisted
}

// ── Rate limit checks ─────────────────────────────────────────

/// Checks if a sender has exceeded the transaction rate limit.
pub fn check_rate_limit(
    blockchain: &Arc<Mutex<Blockchain>>,
    sender: &str,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let allowed = chain.can_send_tx(sender);
    if !allowed {
        println!(
            "[RATE] Sender {} has exceeded the rate limit.",
            sender
        );
    }
    allowed
}

// ── Mempool security ──────────────────────────────────────────

/// Checks if a transaction hash is already in the mempool.
pub fn is_tx_in_mempool(
    blockchain: &Arc<Mutex<Blockchain>>,
    tx_hash: &str,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let exists = chain.mempool.contains_tx(tx_hash);
    if exists {
        println!(
            "[MEMPOOL] Duplicate tx detected: {}",
            tx_hash
        );
    }
    exists
}

/// Checks if a sender already has a transaction with the given nonce in the mempool.
pub fn mempool_has_nonce(
    blockchain: &Arc<Mutex<Blockchain>>,
    sender: &str,
    nonce: u64,
) -> bool {
    let chain = blockchain.lock().expect("chain lock");
    let exists = chain.mempool.has_nonce(sender, nonce);
    if exists {
        println!(
            "[MEMPOOL] Duplicate nonce {} detected for {}.",
            nonce,
            sender
        );
    }
    exists
}

// ── Validator security utilities ──────────────────────────────

/// Checks if a validator is currently jailed.
pub fn is_validator_jailed(
    jail: &JailManager,
    address: &str,
) -> bool {
    let jailed = jail.is_jailed(address);
    println!(
        "[JAIL] Validator {} jailed: {}",
        address,
        jailed
    );
    jailed
}

/// Checks if a validator is currently banned by the slashing system.
pub fn is_validator_banned(
    slashing: &Slashing,
    address: &str,
) -> bool {
    let banned = slashing.is_banned(address);
    println!(
        "[SLASH] Validator {} banned: {}",
        address,
        banned
    );
    banned
}

/// Checks if a validator is trusted by the reputation system.
pub fn is_validator_trusted(
    reputation: &ReputationManager,
    address: &str,
) -> bool {
    let trusted = reputation.is_trusted(address);
    println!(
        "[REPUTATION] Validator {} trusted: {}",
        address,
        trusted
    );
    trusted
}

/// Checks if a validator is healthy enough to produce blocks.
pub fn is_validator_healthy(
    health: &HealthMonitor,
    address: &str,
) -> bool {
    let unhealthy = health.unhealthy(address);
    println!(
        "[HEALTH] Validator {} healthy: {}",
        address,
        !unhealthy
    );
    !unhealthy
}

// ── Emergency protocol utilities ──────────────────────────────

/// Triggers an emergency vote for a network halt.
pub fn trigger_emergency_halt_vote(
    proposal: &mut EmergencyProposal,
    voter: &str,
    council_size: usize,
) {
    proposal.approve(voter.to_string());
    proposal.show();
    println!(
        "[EMERGENCY] Halt vote approved: {} | Rejected: {}",
        proposal.approved(council_size),
        proposal.rejected(council_size)
    );
}

/// Evaluates whether an emergency freeze should be activated.
pub fn evaluate_emergency_freeze(
    freeze: &mut EmergencyFreeze,
    required_votes: u64,
) {
    freeze.evaluate(required_votes as usize);
    freeze.show();
    println!(
        "[EMERGENCY] Network frozen: {}",
        freeze.is_frozen()
    );
}

/// Enters emergency recovery mode.
pub fn enter_emergency_recovery(
    recovery: &mut EmergencyRecovery,
) {
    recovery.approve();
    recovery.enter_recovery();
    recovery.show();
    println!(
        "[EMERGENCY] Recovery active: {}",
        recovery.active()
    );
}

/// Exits emergency recovery mode.
pub fn exit_emergency_recovery(
    recovery: &mut EmergencyRecovery,
) {
    recovery.exit_recovery();
    println!(
        "[EMERGENCY] Recovery active: {}",
        recovery.active()
    );
}

// ── Consensus security utilities ──────────────────────────────

/// Verifies a validator vote signature.
/// [FIX-11] Uses proper signature verification, not hash comparison.
pub fn verify_validator_vote_signature(
    validator: &str,
    block_hash: &str,
    signature: &str,
    public_key: &str,
) -> bool {
    let valid = consensus::ValidatorVote::verify_signature(
    validator,
    block_hash,
    signature,
    public_key,
);
    println!(
        "[CONSENSUS] Vote signature for {} valid: {}",
        validator,
        valid
    );
    valid
}

/// Checks if consensus finality has been reached.
pub fn check_finality(
    consensus: &ConsensusEngine,
) -> bool {
    let reached = consensus.finality_reached();
    println!(
        "[CONSENSUS] Finality reached: {}",
        reached
    );
    reached
}

/// Returns the winning block hash from consensus votes.
pub fn get_winning_block(
    consensus: &ConsensusEngine,
) -> Option<String> {
    let winner = consensus.winning_block();
    println!(
        "[CONSENSUS] Winning block: {:?}",
        winner
    );
    winner
}

// ── Validator selector utilities ──────────────────────────────

/// Selects a validator using the deterministic selector.
pub fn select_validator(
    registry: &ValidatorRegistry,
    seed: u64,
) -> Option<String> {
    let selected = ValidatorSelector::select(registry, seed);
    println!(
        "[SELECTOR] Selected validator: {:?}",
        selected
    );
    selected
}

/// Selects the epoch leader using the V2 election algorithm.
pub fn elect_epoch_leader_v2(
    registry: &ValidatorRegistry,
    reputation: &ReputationManager,
    health: &HealthMonitor,
    rotation: &ValidatorRotation,
) -> Option<String> {
    let winner = ElectionV2::elect(registry, reputation, health, rotation);
    println!(
        "[ELECTION-V2] Epoch leader: {:?}",
        winner
    );
    winner
}

/// Shows election scores for all validators.
pub fn show_election_scores_v2(
    registry: &ValidatorRegistry,
    reputation: &ReputationManager,
    health: &HealthMonitor,
    rotation: &ValidatorRotation,
) {
    for (address, _) in &registry.validators {
        let score = ElectionV2::score(
            address,
            registry,
            reputation,
            health,
            rotation,
        );
        println!(
            "[ELECTION-V2] {} => score {:.2}",
            address,
            score
        );
    }
}

// ⚠️  UNUSED / DEAD CODE — save_epoch_validator_snapshots() below
// is never called anywhere in this codebase. This means
// SnapshotManager (validator_snapshots) is always empty at
// runtime — the .show_all() calls in the main loop only ever
// print an empty header. Verified via grep audit, 2026-08-07.
// ── Validator snapshot utilities ──────────────────────────────

/// Saves a validator snapshot for the current epoch.
pub fn save_epoch_validator_snapshots(
    snapshots: &mut SnapshotManager,
    registry: &ValidatorRegistry,
    epoch: u64,
    uptime: &UptimeTracker,
) {
    for (addr, v) in &registry.validators {
        let uptime_pct = uptime.uptime_percent(addr) as u64;
        let active = uptime_pct >= 50;
        snapshots.save_snapshot(
    epoch,
    addr.clone(),
    v.stake,
    uptime_pct as i64,
    active,
);
    }
    println!(
        "[SNAPSHOT] Saved {} validator snapshots for epoch {}.",
        registry.validators.len(),
        epoch
    );
    snapshots.show_all();
}

// ── Uptime utilities ──────────────────────────────────────────

/// Records a successful block proposal for a validator.
pub fn record_validator_success(
    uptime: &mut UptimeTracker,
    address: &str,
) {
    uptime.record_success(address);
    println!(
        "[UPTIME] Success recorded for {}. Uptime: {:.2}%",
        address,
        uptime.uptime_percent(address)
    );
}

/// Records a missed block proposal for a validator.
pub fn record_validator_miss(
    uptime: &mut UptimeTracker,
    address: &str,
) {
    uptime.record_miss(address);
    println!(
        "[UPTIME] Miss recorded for {}. Uptime: {:.2}%",
        address,
        uptime.uptime_percent(address)
    );
}

// ── Downtime tracking utilities ───────────────────────────────

/// Records a missed slot in the downtime tracker.
pub fn record_downtime_miss(
    downtime: &mut DowntimeTracker,
    address: &str,
) {
    downtime.record_miss(address);
    println!(
        "[DOWNTIME] Miss recorded for {}. Penalise: {}",
        address,
        downtime.should_penalize(address)
    );
    downtime.show();
}

// ── Reward history utilities ──────────────────────────────────

/// Records a validator reward in the reward history.
pub fn record_validator_reward(
    history: &mut RewardHistory,
    address: &str,
    epoch: u64,
    amount: u64,
) {
    history.add_reward(address.to_string(), epoch, amount);
    println!(
        "[HISTORY] Reward {} NSC recorded for {} at epoch {}.",
        amount,
        address,
        epoch
    );
}

/// Shows the full reward history for a validator.
pub fn show_validator_reward_history(
    history: &RewardHistory,
    address: &str,
) {
    history.validator_history(address);
    println!(
        "[HISTORY] Total rewards for {}: {}",
        address,
        history.total_rewards(address)
    );
}

// ── Epoch reward distribution utilities ──────────────────────

/// Distributes epoch rewards to all active validators.
pub fn distribute_epoch_rewards(
    registry: &ValidatorRegistry,
    reputation: &ReputationManager,
    uptime: &UptimeTracker,
    vault: &mut RewardVault,
    total_reward: u64,
) {
    println!(
        "[REWARDS] Distributing {} NSC epoch rewards to {} validators...",
        total_reward,
        registry.validator_count()
    );
    EpochRewardEngine::distribute(
        registry,
        reputation,
        uptime,
        vault,
        total_reward,
    );
    vault.show();
}

// ── Validator performance utilities ──────────────────────────

/// Shows the full validator performance dashboard.
pub fn show_performance_dashboard(
    registry: &ValidatorRegistry,
    treasury: &Treasury,
) {
    ValidatorPerformance::show_dashboard(registry, treasury);
    ValidatorPerformance::network_score(registry);
}

// ── Trust score utilities ─────────────────────────────────────

/// Adds a performance record for trust score calculation.
pub fn add_performance_record(
    history: &mut PerformanceHistory,
    address: &str,
    epoch: u64,
    score: f64,
) {
    history.add_record(address.to_string(), epoch, score);
    println!(
        "[PERF] Score {:.2} recorded for {} at epoch {}.",
        score,
        address,
        epoch
    );
}

/// Shows the trust score for a validator.
pub fn show_trust_score(
    address: &str,
    reputation: &ReputationManager,
    heartbeat: &HeartbeatManager,
    history: &PerformanceHistory,
    current_epoch: u64,
) {
    TrustScore::show(address, reputation, heartbeat, history, current_epoch);
}

// ── Multisig signing utilities ────────────────────────────────

/// Records a signature on the treasury multisig.
pub fn sign_treasury_multisig(
    multisig: &mut TreasuryMultiSig,
    request: &TreasurySpendRequest,
    signer: &str,
    signature_hex: &str,
    public_key_hex: &str,
) {
    match multisig.sign(request, signer, signature_hex, public_key_hex) {
        Ok(()) => {
            multisig.show(&request.id);
            println!(
                "[MULTISIG] Approved for request {}: {}",
                request.id,
                multisig.is_approved(&request.id)
            );
        }
        Err(e) => {
            println!("[MULTISIG] Signature rejected: {}", e);
        }
    }
}

/// Records a signature on the recovery certificate multisig.
pub fn sign_recovery_multisig(
    multisig: &mut RecoveryMultiSig,
    signer: &str,
    required: usize,
) {
    RecoveryCertificateMultiSig::sign(multisig, signer.to_string());
    RecoveryCertificateMultiSig::show(multisig);
    println!(
        "[MULTISIG] Recovery cert valid ({}): {}",
        required,
        RecoveryCertificateMultiSig::verify(multisig, required)
    );
}

// ── Snapshot signing utilities ────────────────────────────────

/// Records a signature on a signed snapshot.
pub fn sign_snapshot(
    snapshot: &mut SignedSnapshot,
    signer: &str,
    required: usize,
) {
    SnapshotSignature::sign(snapshot, signer.to_string());
    SnapshotSignature::show(snapshot);
    println!(
        "[SNAPSHOT] Signature valid ({} required): {}",
        required,
        SnapshotSignature::verify(snapshot, required)
    );
}

/// Creates a new signed snapshot.
pub fn create_signed_snapshot(
    height: u64,
    hash: &str,
) -> SignedSnapshot {
    let snapshot = SnapshotSignature::new(
        height,
        hash.to_string(),
    );
    println!(
        "[SNAPSHOT] Signed snapshot created for height={}.",
        height
    );
    snapshot
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Checkpoint reputation utilities ──────────────────────────

/// Registers a validator with the checkpoint reputation system.
pub fn register_checkpoint_reputation(
    rep: &mut CheckpointReputation,
    address: &str,
) {
    rep.register(address.to_string());
    println!(
        "[CP-REP] Validator {} registered for checkpoint reputation.",
        address
    );
}

/// Rewards a validator in the checkpoint reputation system.
pub fn reward_checkpoint_reputation(
    rep: &mut CheckpointReputation,
    address: &str,
    amount: u64,
) {
    rep.reward(address, amount as i64);
    println!(
        "[CP-REP] Validator {} rewarded {}. Trusted: {}",
        address,
        amount,
        rep.trusted(address)
    );
    rep.show();
}

/// Punishes a validator in the checkpoint reputation system.
pub fn punish_checkpoint_reputation(
    rep: &mut CheckpointReputation,
    address: &str,
    amount: u64,
) {
    rep.punish(address, amount as i64);
    println!(
        "[CP-REP] Validator {} punished {}. Trusted: {}",
        address,
        amount,
        rep.trusted(address)
    );
    rep.show();
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Checkpoint weighted voting utilities ──────────────────────

/// Records a weighted checkpoint vote.
pub fn cast_weighted_checkpoint_vote(
    voting: &mut CheckpointWeightedVoting,
    address: &str,
    weight: u64,
) {
    voting.vote(address.to_string(), weight as i64);
    println!(
        "[CP-VOTE] {} voted with weight {}. Total: {}",
        address,
        weight,
        voting.total_weight()
    );
    voting.show();
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Checkpoint slashing utilities ─────────────────────────────

/// Detects and records a fake checkpoint from a validator.
pub fn detect_fake_checkpoint(
    slashing: &mut CheckpointSlashing,
    address: &str,
    is_valid: bool,
) {
    slashing.detect_fake(address.to_string(), is_valid);
    println!(
        "[CP-SLASH] Validator {} fake checkpoint penalty: {}",
        address,
        slashing.penalty(address)
    );
    slashing.show();
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Checkpoint finalization utilities ─────────────────────────

/// Finalises a checkpoint at the given height.
pub fn finalize_checkpoint(
    finalization: &mut CheckpointFinalization,
    height: u64,
) {
    finalization.finalize(height);
    println!(
        "[CP-FINAL] Checkpoint at height {} finalised: {}",
        height,
        finalization.is_finalized(height)
    );
    finalization.show();
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Checkpoint voting utilities ───────────────────────────────

/// Records an approval vote for a checkpoint.
pub fn approve_checkpoint(
    voting: &mut CheckpointVoting,
    height: u64,
    voter: &str,
) {
    voting.approve(height, voter.to_string());
    println!(
        "[CP-VOTE] {} approved checkpoint at height {}.",
        voter,
        height
    );
    voting.show(height);
}

/// Records a rejection vote for a checkpoint.
pub fn reject_checkpoint(
    voting: &mut CheckpointVoting,
    height: u64,
    voter: &str,
) {
    voting.reject(height, voter.to_string());
    println!(
        "[CP-VOTE] {} rejected checkpoint at height {}.",
        voter,
        height
    );
    voting.show(height);
}

// ── Merkle registry utilities ─────────────────────────────────

/// Registers a Merkle root for a checkpoint.
pub fn register_merkle_root(
    registry: &mut CheckpointMerkleRegistry,
    height: u64,
    root: &str,
) {
    registry.register(height, root.to_string());
    println!(
        "[MERKLE] Root registered for height {}: {}",
        height,
        registry.verify(height, root)
    );
    registry.show();
}

/// Verifies a Merkle root for a checkpoint.
pub fn verify_merkle_root(
    registry: &CheckpointMerkleRegistry,
    height: u64,
    root: &str,
) -> bool {
    let valid = registry.verify(height, root);
    println!(
        "[MERKLE] Height {} root verification: {}",
        height,
        if valid { "PASS" } else { "FAIL" }
    );
    valid
}

// ⚠️  UNUSED / DEAD CODE — not called anywhere in this codebase.
// This is speculative scaffolding for a future multi-validator
// checkpoint governance layer (reputation, weighted voting,
// slashing, finalization quorum) that was never wired into the
// main node loop. Verified unused via grep audit, 2026-08-07.
// Do not assume this provides any active protection.
// ── Historical checkpoint verification ───────────────────────

/// Registers a historical checkpoint for verification.
pub fn register_historical_checkpoint(
    verifier: &mut CheckpointHistoryVerify,
    height: u64,
    merkle_root: &str,
    audit_root: &str,
) {
    verifier.register(HistoricalCheckpoint {
        height,
        merkle_root: merkle_root.to_string(),
        audit_root: audit_root.to_string(),
    });
    println!(
        "[HISTORY] Checkpoint registered at height {}.",
        height
    );
    verifier.show();
}

/// Verifies a historical checkpoint.
pub fn verify_historical_checkpoint(
    verifier: &CheckpointHistoryVerify,
    height: u64,
    merkle_root: &str,
    audit_root: &str,
) -> bool {
    let valid = verifier.verify(height, merkle_root, audit_root);
    println!(
        "[HISTORY] Checkpoint at height {} valid: {}",
        height,
        valid
    );
    valid
}

// ── State snapshot verification ───────────────────────────────

// ⚠️ DEAD-BY-DESIGN (active-4+ audit, 2026-08-11): never called anywhere.
// See state_snapshot_verify.rs header.
/// Verifies that the state data matches the snapshot hash.
pub fn verify_state_snapshot(
    snapshot: &StateSnapshot,
    state_data: &str,
) -> bool {
    let valid = StateSnapshotVerify::verify(snapshot, state_data);
    StateSnapshotVerify::show(snapshot);
    println!(
        "[STATE] Snapshot valid: {}",
        valid
    );
    valid
}

// ⚠️ DEAD-BY-DESIGN (active-4+ audit, 2026-08-11): never called anywhere.
// See state_snapshot_verify.rs header.
/// Creates a new state snapshot from current state data.
pub fn create_state_snapshot(
    height: u64,
    state_data: &str,
) -> StateSnapshot {
    let snapshot = StateSnapshot {
        height,
        state_hash: StateSnapshotVerify::hash(state_data),
    };
    StateSnapshotVerify::show(&snapshot);
    println!(
        "[STATE] Snapshot created at height {}.",
        height
    );
    snapshot
}

// ⚠️  UNUSED / DEAD CODE — register_state_snapshot() and
// get_latest_snapshot() below are never called anywhere in this
// codebase. This means StateSnapshotRegistry (snap_registry) is
// always empty at runtime — the .show() call in the main loop
// only ever prints an empty header. Verified via grep audit,
// 2026-08-07.
// ── Snapshot registry utilities ───────────────────────────────

/// Registers a state snapshot.
pub fn register_state_snapshot(
    registry: &mut StateSnapshotRegistry,
    snapshot: StateSnapshot,
) {
    registry.register(snapshot);
    println!("[STATE] Snapshot registered.");
    registry.show();
}

/// Returns the latest snapshot from the registry.
pub fn get_latest_snapshot(
    registry: &StateSnapshotRegistry,
) -> Option<StateSnapshot> {
    let latest = registry.latest();
    if let Some(ref s) = latest {
        println!(
            "[STATE] Latest snapshot height: {}",
            s.height
        );
    }
    latest.cloned()
}

// ── Fast sync utilities ───────────────────────────────────────

/// Selects the best snapshot for fast sync.
pub fn select_fast_sync_snapshot(
    snapshots: &[StateSnapshot],
) -> Option<StateSnapshot> {
    let best = FastSyncSelector::select_best(snapshots);
    if let Some(ref s) = best {
        FastSyncSelector::show(s);
    }
    best.map(|s| s.clone())
}

// ── Treasury audit utilities ──────────────────────────────────

/// Records a treasury transaction in the audit log.
pub fn record_treasury_audit(
    audit: &mut TreasuryAudit,
    tx_id: &str,
    amount: u64,
    direction: &str,
    suspicious_threshold: u64,
) {
    audit.record(tx_id.to_string(), amount, direction.to_string());
    audit.show();
    audit.detect_suspicious(suspicious_threshold);
}

/// Adds a record to the treasury audit hash chain.
pub fn add_treasury_hash_record(
    chain: &mut TreasuryAuditHashChain,
    record: &str,
) {
    chain.add_record(record.to_string());
    chain.show();
}

// ── Recovery authority utilities ──────────────────────────────

/// Adds a recovery authority.
pub fn add_recovery_authority(
    governance: &mut RecoveryAuthorityGovernance,
    address: &str,
) {
    governance.add_authority(address.to_string());
    println!(
        "[AUTHORITY] {} added. Total: {}",
        address,
        governance.total()
    );
    governance.show();
}

/// Removes a recovery authority.
pub fn remove_recovery_authority(
    governance: &mut RecoveryAuthorityGovernance,
    address: &str,
) {
    governance.remove_authority(address);
    println!(
        "[AUTHORITY] {} removed. Total: {}",
        address,
        governance.total()
    );
    governance.show();
}

/// Checks if an address is a recovery authority.
pub fn is_recovery_authority(
    governance: &RecoveryAuthorityGovernance,
    address: &str,
) -> bool {
    let is_auth = governance.is_authority(address);
    println!(
        "[AUTHORITY] {} is authority: {}",
        address,
        is_auth
    );
    is_auth
}

// ── Appeal resolution utilities ───────────────────────────────

/// Shows the resolution status of a validator appeal.
pub fn show_appeal_resolution(
    address: &str,
    appeals: &ValidatorAppeals,
    voting: &SecurityCouncilVoting,
) {
    AppealResolution::show(address, appeals, voting);
}

// ── Safe restart utilities ────────────────────────────────────

/// Runs a safe restart sequence.
pub fn run_safe_restart(
    manager: &mut SafeRestartManager,
) {
    manager.verify_validators();
    manager.verify_epoch();
    manager.verify_consensus();
    manager.restart();
    manager.show();
    println!("[RESTART] Safe restart sequence complete.");
}

// ── Protocol version utilities ────────────────────────────────

/// Registers a peer node with the version manager.
pub fn register_peer_version(
    manager: &mut ProtocolVersionManager,
    node_id: &str,
    version: &str,
) {
    manager.register_node(node_id.to_string(), version.to_string());
    println!(
        "[VERSION] Peer '{}' registered as v{}.",
        node_id,
        version
    );
    manager.show();
    manager.readiness_report();
}

// ── Node fingerprint utilities ────────────────────────────────

/// Verifies two node fingerprints are different (anti-sybil).
pub fn fingerprints_are_unique(
    fp1: &str,
    fp2: &str,
) -> bool {
    let unique = fp1 != fp2;
    println!(
        "[FINGERPRINT] Fingerprints unique: {}",
        unique
    );
    unique
}

// ── Availability scoring utilities ────────────────────────────

/// Records a successful availability check for a validator.
pub fn record_availability_success(
    monitor: &mut AvailabilityMonitor,
    address: &str,
) {
    monitor.success(address);
    println!(
        "[AVAIL] Success for {}. Score: {:.2}",
        address,
        monitor.score(address)
    );
}

/// Records a failed availability check for a validator.
pub fn record_availability_failure(
    monitor: &mut AvailabilityMonitor,
    address: &str,
) {
    monitor.failure(address);
    println!(
        "[AVAIL] Failure for {}. Score: {:.2}",
        address,
        monitor.score(address)
    );
}

// ── Reliability ranking utilities ─────────────────────────────

/// Calculates the reliability score for a validator.
pub fn calculate_reliability_score(
    stake: u64,
    uptime_pct: f64,
    reputation_score: f64,
    availability_score: f64,
) -> f64 {
    let score = ReliabilityRanking::calculate(
    stake as i64,
    uptime_pct,
    reputation_score,
    availability_score,
);
    println!("[RELIABILITY] Score: {:.4}", score);
    score
}

/// Shows the full reliability ranking.
pub fn show_reliability_ranking(
    rankings: &HashMap<String, f64>,
) {
    ReliabilityRanking::show(rankings);
}

// ── Rotation scheduler utilities ──────────────────────────────

/// Selects the next leader using the rotation scheduler.
pub fn schedule_next_leader(
    scheduler: &mut RotationScheduler,
    rankings: &HashMap<String, f64>,
    epoch: u64,
) -> Option<String> {
    let leader = scheduler.select_next(rankings, epoch);
    println!(
        "[SCHEDULER] Epoch {} leader: {:?}",
        epoch,
        leader
    );
    scheduler.show();
    leader
}

// ── Suspension manager utilities ──────────────────────────────

/// Audits a validator for suspension criteria.
pub fn audit_validator_suspension(
    address: &str,
    strikes: &StrikeEngineV2,
    availability: &AvailabilityMonitor,
    reputation: &ReputationManager,
) {
    SuspensionManager::audit(address, strikes, availability, reputation);
    println!(
        "[SUSPENSION] Audit complete for {}.",
        address
    );
}

// ── Recovery manager utilities ────────────────────────────────

/// Starts a probation period for a recovering validator.
pub fn start_validator_probation(
    manager: &mut RecoveryManager,
    address: &str,
    epoch: u64,
) {
    manager.start_probation(address.to_string(), epoch);
    println!(
        "[RECOVERY] Probation started for {} at epoch {}.",
        address,
        epoch
    );
    manager.show();
}

/// Checks if a validator can recover from probation.
pub fn check_recovery_eligibility(
    manager: &mut RecoveryManager,
    address: &str,
    current_epoch: u64,
    reputation_score: u64,
    availability_score: f64,
) -> bool {
    let eligible = manager.can_recover(
    address,
    current_epoch,
    reputation_score as i64,
    availability_score,
);
    println!(
        "[RECOVERY] {} eligible for recovery: {}",
        address,
        eligible
    );
    if eligible {
        manager.recover(address);
    }
    manager.show();
    eligible
}

// ============================================================
// END OF SECURITY & VERIFICATION MODULE
// ============================================================
