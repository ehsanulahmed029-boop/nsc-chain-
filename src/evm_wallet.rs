// ============================================================
// NUSACOIN (NSC) — evm_wallet.rs
// EVM-compatible (secp256k1 + Keccak256) wallet primitives.
// This module is additive: it does not touch the existing
// Ed25519-based wallet.rs, so the current live chain and its
// balances are unaffected until this is wired into consensus.
// ============================================================

use secp256k1::{Secp256k1, SecretKey, PublicKey};
use sha3::{Digest, Keccak256};

/// A freshly generated EVM-style keypair with its derived address.
pub struct EvmWallet {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
    pub address: String,
}

/// Derive an Ethereum-style 0x... address from an uncompressed
/// secp256k1 public key: keccak256(pubkey[1:])[12..32].
pub fn eth_address_from_pubkey(pubkey: &PublicKey) -> String {
    let uncompressed = pubkey.serialize_uncompressed(); // 65 bytes, leading 0x04
    let mut hasher = Keccak256::new();
    hasher.update(&uncompressed[1..]); // drop the 0x04 prefix byte
    let hash = hasher.finalize();
    let addr_bytes = &hash[12..]; // last 20 bytes
    format!("0x{}", hex::encode(addr_bytes))
}

/// Generate a brand new EVM-compatible wallet.
/// Uses `getrandom` directly (bypassing the `rand` trait ecosystem)
/// to avoid rand_core version conflicts between this project's
/// existing `rand 0.8` dependency and secp256k1's `rand 0.9`.
pub fn generate_evm_wallet() -> EvmWallet {
    let secp = Secp256k1::new();
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).expect("system RNG failure");
    let secret_key = SecretKey::from_byte_array(seed)
        .expect("32 random bytes are always a valid secp256k1 secret key (astronomically unlikely all-zero case aside)");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let address = eth_address_from_pubkey(&public_key);
    EvmWallet { secret_key, public_key, address }
}

/// Reconstruct a wallet (and its address) from a known 32-byte
/// secret key, e.g. one supplied via CLI or config for testing.
pub fn wallet_from_secret_bytes(bytes: &[u8; 32]) -> Result<EvmWallet, secp256k1::Error> {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_byte_array(*bytes)?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let address = eth_address_from_pubkey(&public_key);
    Ok(EvmWallet { secret_key, public_key, address })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_looking_address() {
        let wallet = generate_evm_wallet();
        assert!(wallet.address.starts_with("0x"));
        assert_eq!(wallet.address.len(), 42); // "0x" + 40 hex chars
        println!("Generated EVM address: {}", wallet.address);
    }
}
