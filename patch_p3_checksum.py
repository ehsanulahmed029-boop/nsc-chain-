import sys

path = "/root/nsc-chain/src/wallet.rs"
with open(path, "r") as f:
    content = f.read()

anchor = '''    pub fn derive_address(public_key: &VerifyingKey) -> String {
        format!(
            "NSC{}",
            &hex::encode(public_key.as_bytes())[..16]
        )
    }
'''

count = content.count(anchor)
if count != 1:
    print(f"FATAL: anchor found {count} times, expected 1. Aborting.")
    sys.exit(1)

replacement = anchor + '''
    // ── Address checksum (opt-in, non-breaking) [P3-FIX 2026-08-16] ──
    //
    // derive_address() above is UNCHANGED and still returns a plain
    // lowercase address, exactly as before — every existing address
    // in the system (including the operator wallet) remains valid
    // and unaffected. This section adds an OPTIONAL EIP-55-style
    // mixed-case checksum on top, so frontends/wallets can offer
    // typo detection without any migration or format break:
    //
    //   - checksum_address(): takes a plain "NSC<16 hex>" address and
    //     returns a mixed-case version encoding a checksum.
    //   - verify_address_checksum(): validates an address. All-
    //     lowercase addresses (the legacy/current format) are always
    //     treated as valid (no checksum was ever applied to them).
    //     Mixed-case addresses are checked against their checksum and
    //     rejected if it doesn't match — this is what actually catches
    //     a typo'd or corrupted address before funds are sent to it.
    //
    // Nothing elsewhere in the codebase calls these yet; wiring them
    // into a wallet UI's "send" flow (checksum on address display,
    // verify on paste/entry) is a follow-up frontend task, not part
    // of this backend hardening pass.

    /// Returns a mixed-case checksummed version of `address`, using
    /// the address's own hex portion hashed via SHA-256 to decide
    /// the case of each hex character (EIP-55-style). Non-hex
    /// characters (like the "NSC" prefix) are left as-is.
    pub fn checksum_address(address: &str) -> String {
        let hex_part = match address.strip_prefix("NSC") {
            Some(h) => h,
            None => return address.to_string(),
        };

        let lower = hex_part.to_lowercase();
        let hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(lower.as_bytes());
            hasher.finalize()
        };
        let hash_hex = hex::encode(hash);

        let mut out = String::from("NSC");
        for (i, c) in lower.chars().enumerate() {
            if c.is_ascii_hexdigit() && c.is_alphabetic() {
                // Use the corresponding nibble of the hash to decide case.
                let hash_char = hash_hex.as_bytes()[i % hash_hex.len()] as char;
                let hash_val = hash_char.to_digit(16).unwrap_or(0);
                if hash_val >= 8 {
                    out.push(c.to_ascii_uppercase());
                } else {
                    out.push(c);
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Verifies an address's checksum. All-lowercase addresses are
    /// always accepted (legacy/unchecksummed format, still the
    /// default output of derive_address()). Mixed-case addresses
    /// must match their expected checksum exactly, or this returns
    /// false — catching typos/corruption that plain lowercase
    /// addresses cannot detect.
    pub fn verify_address_checksum(address: &str) -> bool {
        let hex_part = match address.strip_prefix("NSC") {
            Some(h) => h,
            None => return false,
        };

        // All-lowercase (or no alphabetic hex chars at all) — legacy
        // format, always valid since no checksum was ever applied.
        if hex_part.chars().all(|c| !c.is_alphabetic() || c.is_lowercase()) {
            return true;
        }

        Self::checksum_address(address) == address
    }
'''

content = content.replace(anchor, replacement, 1)

with open(path, "w") as f:
    f.write(content)

print("wallet.rs patched successfully (address checksum, non-breaking addition).")
