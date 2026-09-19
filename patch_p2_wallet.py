import sys

path = "/root/nsc-chain/src/wallet.rs"
with open(path, "r") as f:
    lines = f.readlines()

start = 244
end   = 255

segment = "".join(lines[start-1:end])

expected_markers = [
    "Exports the raw private key as hex.",
    "pub fn export_private_key(&self) -> String {",
    "hex::encode(self.private_key.to_bytes())",
]

for marker in expected_markers:
    if marker not in segment:
        print(f"FATAL: expected marker not found in lines {start}-{end}: {marker}")
        sys.exit(1)

print(f"Verified lines {start}-{end} contain expected content. Proceeding.")

new_fn = '''    /// Exports the raw private key as hex.
    ///
    /// [P2-FIX 2026-08-16] Previous header comment claimed this
    /// "requires explicit opt-in and is disabled in production
    /// builds" but the code had no such gate — any caller could
    /// exfiltrate the raw key unconditionally. This function is
    /// currently unreachable from any live code path (confirmed via
    /// codebase-wide grep, 2026-08-15 audit), but now enforces a
    /// real opt-in in case it is ever wired into a caller later.
    ///
    /// `confirm` must be explicitly passed as `true` by the caller —
    /// there is no default, so a careless call site cannot
    /// accidentally export a key. Returns None if confirm is false.
    ///
    /// The caller is still responsible for encrypting the result
    /// before storing or transmitting it. NEVER call this in a log
    /// statement or API response.
    pub fn export_private_key(&self, confirm: bool) -> Option<String> {
        if !confirm {
            eprintln!(
                "[WALLET] export_private_key() called without explicit confirm=true. Refusing."
            );
            return None;
        }
        // In production this should also be wrapped with a
        // passphrase-protected encryption layer before use.
        Some(hex::encode(self.private_key.to_bytes()))
    }
'''

new_lines = lines[:start-1] + [new_fn] + lines[end:]

with open(path, "w") as f:
    f.writelines(new_lines)

print("wallet.rs patched successfully (export_private_key opt-in gate).")
