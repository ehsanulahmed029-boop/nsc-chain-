// ============================================================
// NUSACOIN (NSC) — evm_tx.rs
// Decodes legacy (pre-EIP-1559) Ethereum-style transactions as
// sent by MetaMask/Binance Wallet, recovers the sender address
// from the ECDSA signature, and exposes the fields needed to
// apply a native NSC transfer. Fully separate from the existing
// Ed25519-based Transaction type in transaction.rs.
// ============================================================

use rlp::{Rlp, RlpStream};
use secp256k1::{Secp256k1, Message, ecdsa::{RecoveryId, RecoverableSignature}};
use sha3::{Digest, Keccak256};

/// NSC's EVM-facing chain ID. Single source of truth —
/// transaction.rs and evm_rpc.rs both use this constant.
pub const NSC_EVM_CHAIN_ID: u64 = 7788;

pub struct LegacyEthTx {
    pub nonce:     u64,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub to:        Option<[u8; 20]>,
    pub value:     u128,
    pub data:      Vec<u8>,
    pub v:         u64,
    pub r:         Vec<u8>,
    pub s:         Vec<u8>,
}

#[derive(Debug)]
pub enum EvmTxError {
    Decode(String),
    Recover(String),
}

impl std::fmt::Display for EvmTxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvmTxError::Decode(s) => write!(f, "decode error: {}", s),
            EvmTxError::Recover(s) => write!(f, "recover error: {}", s),
        }
    }
}

fn be_to_u128(bytes: &[u8]) -> u128 {
    let mut buf = [0u8; 16];
    let len = bytes.len().min(16);
    buf[16 - len..].copy_from_slice(&bytes[bytes.len() - len..]);
    u128::from_be_bytes(buf)
}

fn be_to_u64(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let len = bytes.len().min(8);
    buf[8 - len..].copy_from_slice(&bytes[bytes.len() - len..]);
    u64::from_be_bytes(buf)
}

fn field(rlp: &Rlp, idx: usize) -> Result<Vec<u8>, EvmTxError> {
    rlp.at(idx)
        .map_err(|e| EvmTxError::Decode(e.to_string()))?
        .data()
        .map_err(|e| EvmTxError::Decode(e.to_string()))
        .map(|d| d.to_vec())
}

pub fn decode_legacy_tx(raw: &[u8]) -> Result<LegacyEthTx, EvmTxError> {
    let rlp = Rlp::new(raw);
    if !rlp.is_list() || rlp.item_count().unwrap_or(0) != 9 {
        return Err(EvmTxError::Decode("expected 9-field legacy tx list (EIP-1559/type-2 tx not yet supported)".into()));
    }

    let to_b = field(&rlp, 3)?;
    let to = if to_b.is_empty() {
        None
    } else if to_b.len() == 20 {
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&to_b);
        Some(arr)
    } else {
        return Err(EvmTxError::Decode("invalid 'to' length".into()));
    };

    Ok(LegacyEthTx {
        nonce:     be_to_u64(&field(&rlp, 0)?),
        gas_price: be_to_u128(&field(&rlp, 1)?),
        gas_limit: be_to_u64(&field(&rlp, 2)?),
        to,
        value:     be_to_u128(&field(&rlp, 4)?),
        data:      field(&rlp, 5)?,
        v:         be_to_u64(&field(&rlp, 6)?),
        r:         field(&rlp, 7)?,
        s:         field(&rlp, 8)?,
    })
}

/// Rebuilds the exact RLP payload that was originally signed,
/// hashes it with keccak256, and recovers the sender's 0x...
/// address from the (r, s, recovery_id) signature.
pub fn recover_sender(tx: &LegacyEthTx, expected_chain_id: u64) -> Result<String, EvmTxError> {
    let is_eip155 = tx.v >= 35;

    // [P3-hardening, 2026-08-16] Reject non-EIP-155 (legacy, v=27/28)
    // transactions outright. Legacy-format txs encode no chain_id, so
    // a signed tx meant for another EVM chain (or vice versa) could be
    // replayed here without any way to detect it. Requiring EIP-155
    // closes that cross-chain replay surface, and also removes the
    // v<27 underflow path below (tx.v - 27 on a u64 could otherwise
    // wrap in release builds for a malformed v value).
    if !is_eip155 {
        return Err(EvmTxError::Recover(
            "legacy (non-EIP-155) transactions are not accepted; sign with a chain id".to_string()
        ));
    }

    let chain_id = (tx.v - 35) / 2;

    if chain_id != expected_chain_id {
        return Err(EvmTxError::Recover(format!(
            "chain id mismatch: tx has {}, node expects {}",
            chain_id, expected_chain_id
        )));
    }

    let recovery_id_raw: u8 = (tx.v - chain_id * 2 - 35) as u8;

    let signing_rlp = {
        let mut s = RlpStream::new();
        s.begin_list(if is_eip155 { 9 } else { 6 });
        s.append(&tx.nonce);
        s.append(&tx.gas_price);
        s.append(&tx.gas_limit);
        match &tx.to {
            Some(addr) => { s.append(&addr.as_slice()); },
            None       => { s.append_empty_data(); },
        }
        s.append(&tx.value);
        s.append(&tx.data);
        if is_eip155 {
            s.append(&chain_id);
            s.append_empty_data();
            s.append_empty_data();
        }
        s.out().to_vec()
    };

    let mut hasher = Keccak256::new();
    hasher.update(&signing_rlp);
    let hash = hasher.finalize();

    let recovery_id = RecoveryId::from_u8_masked(recovery_id_raw);

    let mut sig_bytes = [0u8; 64];
    let r_len = tx.r.len().min(32);
    let s_len = tx.s.len().min(32);
    sig_bytes[32 - r_len..32].copy_from_slice(&tx.r[tx.r.len() - r_len..]);
    sig_bytes[64 - s_len..64].copy_from_slice(&tx.s[tx.s.len() - s_len..]);

    let recoverable_sig = RecoverableSignature::from_compact(&sig_bytes, recovery_id)
        .map_err(|e| EvmTxError::Recover(e.to_string()))?;

    let secp = Secp256k1::new();
    let message = Message::from_digest(hash.into());

    let pubkey = secp.recover_ecdsa(message, &recoverable_sig)
        .map_err(|e| EvmTxError::Recover(e.to_string()))?;

    Ok(crate::evm_wallet::eth_address_from_pubkey(&pubkey))
}

// ============================================================
// PERSONAL_SIGN VERIFICATION (EIP-191)
// For authorizing off-chain actions (swap, add-liquidity) with
// a signed message rather than a full transaction. MetaMask's
// `personal_sign` / `eth_sign` prefixes the message with
// "\x19Ethereum Signed Message:\n<len>" before hashing+signing;
// we must replicate that exact prefix to verify correctly.
// ============================================================

/// Verifies that `signature` (65 bytes: r[32] + s[32] + v[1],
/// hex-encoded with or without "0x") was produced by signing
/// `message` with the private key behind `expected_address`
/// (lowercase 0x... EVM address).
pub fn verify_personal_sign(
    message: &str,
    signature_hex: &str,
    expected_address: &str,
) -> bool {
    let sig_hex = signature_hex.trim_start_matches("0x");
    let sig_bytes = match hex::decode(sig_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    if sig_bytes.len() != 65 {
        return false;
    }

    let r = &sig_bytes[0..32];
    let s = &sig_bytes[32..64];
    let v_raw = sig_bytes[64];
    // MetaMask returns v as 27/28 (sometimes 0/1); normalize to 0/1.
    let recovery_id_raw: u8 = if v_raw >= 27 { v_raw - 27 } else { v_raw };

    let recovery_id = match RecoveryId::from_u8_masked(recovery_id_raw) {
        id => id,
    };

    let mut sig_compact = [0u8; 64];
    sig_compact[0..32].copy_from_slice(r);
    sig_compact[32..64].copy_from_slice(s);

    let recoverable_sig = match RecoverableSignature::from_compact(&sig_compact, recovery_id) {
        Ok(s) => s,
        Err(_) => return false,
    };

    // EIP-191 personal_sign prefix.
    let prefixed = format!("\x19Ethereum Signed Message:\n{}{}", message.len(), message);
    let mut hasher = Keccak256::new();
    hasher.update(prefixed.as_bytes());
    let hash = hasher.finalize();

    let msg = Message::from_digest(hash.into());

    let secp = Secp256k1::new();
    let pubkey = match secp.recover_ecdsa(msg, &recoverable_sig) {
        Ok(pk) => pk,
        Err(_) => return false,
    };

    let recovered_address = crate::evm_wallet::eth_address_from_pubkey(&pubkey);
    recovered_address.to_lowercase() == expected_address.to_lowercase()
}
