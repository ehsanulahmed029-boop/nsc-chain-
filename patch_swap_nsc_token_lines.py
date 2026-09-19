import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    lines = f.readlines()

expected = {
    2256: "save_evm_state(&chain.balances, &chain.nonces)",
    2260: "save_pool(pool.0, pool.1)",
    2265: "save_token_pool(to, tpool.0, tpool.1)",
    2271: "save_tokens(&tokens)",
}
for lineno, needle in expected.items():
    actual = lines[lineno - 1]
    if needle not in actual:
        print(f"ERROR: line {lineno} does not contain expected text.")
        print(f"  expected substring: {needle}")
        print(f"  actual line: {actual!r}")
        sys.exit(1)
print("All four anchor lines verified.")

idx_2256 = 2256 - 1
idx_2260 = 2260 - 1
idx_2265 = 2265 - 1
idx_2271 = 2271 - 1
idx_2272 = 2272 - 1  # log_trade("NSC", to, ...) line, insert batch block before this

indent = "                                                  "  # matches indent at this depth

batch_block = (
    indent + "// [ATOMICITY FIX] evm_state.json, pool.json, the token's\n" +
    indent + "// pool file, and tokens.json committed together instead\n" +
    indent + "// of four sequential independent saves.\n" +
    indent + "let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();\n" +
    indent + "batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));\n" +
    indent + "if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {\n" +
    indent + "    batch_writes.push(w);\n" +
    indent + "}\n" +
    indent + "if let Some(w) = crate::storage::prepare_token_pool_write(to, tpool.0, tpool.1) {\n" +
    indent + "    batch_writes.push(w);\n" +
    indent + "}\n" +
    indent + "if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {\n" +
    indent + "    batch_writes.push(w);\n" +
    indent + "}\n" +
    indent + "if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {\n" +
    indent + "    eprintln!(\"[API] Failed to commit swap batch (NSC->token): {}\", e);\n" +
    indent + "}\n"
)

# Blank out the four old save_* lines, highest index first so earlier
# indices in this same pass stay valid (we're only blanking, not removing,
# so subsequent index references below remain correct either way).
lines[idx_2271] = ""
lines[idx_2265] = ""
lines[idx_2260] = ""
lines[idx_2256] = ""

# chain lock is released after the `{ ... }` block containing save_evm_state
# (line 2257 originally = closing brace). Blanking line 2256 leaves that
# brace intact on the next line, so the lock is still dropped correctly.

lines.insert(idx_2272, batch_block)

with open(path, "w") as f:
    f.writelines(lines)

print("Patched: /swap NSC->USDT->TOKEN router branch now uses atomic_write_batch (line-based patch)")
