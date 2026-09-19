// Standalone wallet generator for treasury multisig owners.
// Mirrors the exact address derivation logic in src/wallet.rs
// (NSC + first 16 hex chars of the Ed25519 public key) so the
// generated addresses will match what TreasuryMultiSig expects.
//
// Private keys are printed to the terminal ONLY. Nothing is
// written to disk or sent anywhere. Copy them out immediately
// and store them offline, encrypted. Do not paste private keys
// into any chat, ticket, or log.

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use hex;

fn derive_address(public_key_hex: &str) -> String {
    format!("NSC{}", &public_key_hex[..16])
}

fn main() {
    println!("========================================================");
    println!(" NUSACOIN TREASURY OWNER WALLET GENERATOR");
    println!(" Generating 3 owner wallets. SAVE PRIVATE KEYS OFFLINE.");
    println!(" These are shown ONCE and not stored anywhere.");
    println!("========================================================\n");

    for i in 1..=3 {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        let public_key_hex = hex::encode(verifying_key.as_bytes());
        let private_key_hex = hex::encode(signing_key.to_bytes());
        let address = derive_address(&public_key_hex);

        println!("── OWNER {} ──────────────────────────────", i);
        println!("Address     : {}", address);
        println!("Public Key  : {}", public_key_hex);
        println!("Private Key : {}   <-- SAVE THIS OFFLINE, SHOW NO ONE", private_key_hex);
        println!();
    }

    println!("========================================================");
    println!(" NEXT STEPS:");
    println!(" 1. Copy each 'Address' value only (not the private key)");
    println!("    into TreasuryMultiSig::new() owners list in main.rs.");
    println!(" 2. Store each private key offline, encrypted, one per");
    println!("    trusted owner. Never store all 3 keys in one place.");
    println!(" 3. Clear this terminal's scrollback after copying:");
    println!("    the 'clear' command does NOT erase scrollback/history.");
    println!("========================================================");
}
