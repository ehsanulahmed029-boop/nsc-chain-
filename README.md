# NusaCoin (NSC)

A custom Rust-based Layer-1 blockchain, single-node PoW chain running live at nusacoin.online.

## Features

- Custom Layer-1 PoW blockchain (Rust)
- EVM-compatible JSON-RPC layer (Chain ID: 7788)
- AMM DEX with liquidity pools
- BSC bridge with WNSC wrapped token
- L2 optimistic rollup bridge
- Custom token creation platform
- Staking system
- Multisig treasury

## Tech Stack

- Language: Rust
- Networking: Custom P2P + JSON-RPC (EVM-compatible)
- Frontend: HTML/JS wallet interface

## Development

    cargo build --release
    cargo test --release

## CI/CD

Every push to main triggers automated build and test via GitHub Actions.

## Validator Setup

See [VALIDATOR.md](./VALIDATOR.md) for hardware requirements and node setup.

## License

TBD

---

*This project is under active development.*
