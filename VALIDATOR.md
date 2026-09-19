# NSC Validator Guide

This document describes hardware requirements and setup expectations for
running an NSC (NusaCoin) node.

> **Status:** NSC is currently a single-node chain. This document is
> prepared ahead of the multi-node/validator phase and will be revised
> once P2P robustness and BFT-style consensus testing are complete.

## Hardware Requirements (Minimum)

| Component | Minimum | Recommended |
|---|---|---|
| CPU | 2 cores | 4+ cores |
| RAM | 2 GB | 4 GB |
| Storage | 20 GB SSD | 50 GB SSD (NVMe preferred) |
| Network | 10 Mbps up/down, stable | 50+ Mbps, low jitter |
| Uptime | 95%+ | 99%+ |

> **Note:** These figures are placeholders based on the current chain's
> resource footprint (single Android/Termux node). They should be
> revisited once real multi-node load testing exists.

## Software Requirements

- Rust 1.96.0 (pinned via rust-toolchain.toml in this repo)
- Linux or Android (Termux) — Windows untested
- Open inbound port for P2P (default: 6000)
- Open inbound port for RPC/API if publicly serving (default: 8080 API,
  8545 EVM-RPC), typically reverse-proxied via nginx

## Node Setup (Summary)

    git clone https://github.com/ehsanulahmed029-boop/nsc-chain-.git
    cd nsc-chain-
    cargo build --release

Required environment variables at startup:

- NSC_API_KEY
- NSC_API_BIND (e.g. 127.0.0.1:8080)
- NSC_P2P_ADDR (e.g. 127.0.0.1:6000)
- NSC_DATA_DIR
- NSC_STATE_HMAC_KEY — required in every startup path; state
  integrity checks will fail without it
- MORALIS_API_KEY, CMC_API_KEY — optional, only needed for
  BSC token detection / price features

## Validator Responsibilities (Draft — Pre-Multi-Node)

> This section is a placeholder. Concrete staking/slashing/uptime
> rules will be finalized once the validator subsystem
> (EpochRewardEngine, ValidatorRegistry) is wired to real network
> participation. As of now, this subsystem is present in code but
> inert (no live call sites register validators).

- Maintain node uptime above the recommended threshold
- Keep software updated to the latest tagged release
- Do not run two instances of the same validator key concurrently
  (causes state/HMAC desync)

## Chain Parameters

- Chain ID: 7788
- Consensus: Proof of Work (single-node; multi-node consensus model TBD)
- Max Supply: 25,000,000 NSC

---

*This document is a living draft. Sections marked "placeholder" or
"TBD" will be updated as the validator/multi-node phase progresses.*
