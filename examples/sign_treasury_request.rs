// Signs a treasury spend request's canonical message using a
// private key supplied ONLY via environment variable, never as
// a CLI argument (which could leak into shell history) and never
// printed back out. Only the resulting signature_hex is printed —
// that is safe to share, since it proves nothing without the
// public key it was already tied to.

use nsc_chain::wallet::Wallet;
use nsc_chain::multisig::TreasurySpendRequest;
use ed25519_dalek::SigningKey;
use std::env;

fn main() {
    let priv_hex = env::var("NSC_SIGNING_KEY")
        .expect("Set NSC_SIGNING_KEY env var to your private key hex before running this.");

    let id = env::var("NSC_REQ_ID").expect("Set NSC_REQ_ID");
    let amount: u128 = env::var("NSC_REQ_AMOUNT").expect("Set NSC_REQ_AMOUNT").parse().expect("amount must be a number");
    let recipient = env::var("NSC_REQ_RECIPIENT").expect("Set NSC_REQ_RECIPIENT");
    let description = env::var("NSC_REQ_DESC").unwrap_or_default();

    let bytes = hex::decode(&priv_hex).expect("invalid private key hex");
    let arr: [u8; 32] = bytes.try_into().expect("private key must be 32 bytes");
    let signing_key = SigningKey::from_bytes(&arr);
    let public_key = signing_key.verifying_key();
    let address = Wallet::derive_address(&public_key);
    let public_key_hex = hex::encode(public_key.as_bytes());

    let request = TreasurySpendRequest::new(id, amount, recipient, description);
    let message = request.canonical_message();

    use ed25519_dalek::Signer;
    let signature = signing_key.sign(message.as_bytes());
    let signature_hex = hex::encode(signature.to_bytes());

    println!("address        : {}", address);
    println!("public_key_hex : {}", public_key_hex);
    println!("signature_hex  : {}", signature_hex);
    println!();
    println!("(private key was read from env var only, never printed above)");
}
