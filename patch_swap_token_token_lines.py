import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    lines = f.readlines()

# 1-indexed line numbers from `nl` output, converted to 0-indexed for the list.
# Verify the three save-call lines are exactly what we expect before touching anything.
expected = {
    2184: "save_token_pool(from, pool_a.0, pool_a.1)",
    2189: "save_token_pool(to, pool_b.0, pool_b.1)",
    2194: "save_tokens(&tokens)",
}

for lineno, needle in expected.items():
    actual = lines[lineno - 1]
    if needle not in actual:
        print(f"ERROR: line {lineno} does not contain expected text.")
        print(f"  expected substring: {needle}")
        print(f"  actual line: {actual!r}")
        sys.exit(1)

print("All three anchor lines verified.")

# Remove line 2194 (save_tokens) and its blank line 2195 stays; we replace
# line 2194 with nothing (it'll be re-added via batch below), and replace
# line 2189 (save_token_pool to) similarly, and line 2184 (save_token_pool from)
# similarly. We do this from bottom to top so earlier line numbers don't shift.

# Line 2194 (index 2193): save_tokens(&tokens); -> remove, will be replaced by batch block after line 2196 (log_trade)
# Simplest safe approach: blank out the three save_* lines (turn them into no-ops),
# then insert the batch-write block right before the log_trade line (2196).

idx_2184 = 2184 - 1
idx_2189 = 2189 - 1
idx_2194 = 2194 - 1
idx_2196 = 2196 - 1  # log_trade line, insert batch block before this

indent = "                                            "  # matches surrounding indent level

batch_block = (
    indent + "// [ATOMICITY FIX] Both token pool files and tokens.json\n" +
    indent + "// committed together instead of three sequential\n" +
    indent + "// independent saves.\n" +
    indent + "let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();\n" +
    indent + "if let Some(w) = crate::storage::prepare_token_pool_write(from, pool_a.0, pool_a.1) {\n" +
    indent + "    batch_writes.push(w);\n" +
    indent + "}\n" +
    indent + "if let Some(w) = crate::storage::prepare_token_pool_write(to, pool_b.0, pool_b.1) {\n" +
    indent + "    batch_writes.push(w);\n" +
    indent + "}\n" +
    indent + "if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {\n" +
    indent + "    batch_writes.push(w);\n" +
    indent + "}\n" +
    indent + "if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {\n" +
    indent + "    eprintln!(\"[API] Failed to commit swap batch (token->token): {}\", e);\n" +
    indent + "}\n"
)

# Remove the three old save_* lines (set to empty string) — do highest index first
# so we don't shift the indices of the ones we haven't processed yet.
lines[idx_2194] = ""
lines[idx_2189] = ""
lines[idx_2184] = ""

# Insert the batch block right before the log_trade line.
# idx_2196 is still valid since we only blanked lines, didn't remove them from the list.
lines.insert(idx_2196, batch_block)

with open(path, "w") as f:
    f.writelines(lines)

print("Patched: /swap TOKEN_A->TOKEN_B branch now uses atomic_write_batch (line-based patch)")
