import sys

path = "/root/nsc-chain/src/network.rs"
with open(path, "r") as f:
    lines = f.readlines()

# 1-indexed 638-649 -> 0-indexed 637-648 (inclusive start, exclusive end for slice = 649)
start = 638
end   = 649

segment = lines[start-1:end]
joined = "".join(segment)

expected_markers = [
    "IMPORTANT: this appends a single block without",
    "chain.blocks.push(block.clone());",
    "Accepted block {} from peer",
]

for marker in expected_markers:
    if marker not in joined:
        print(f"FATAL: expected marker not found in lines {start}-{end}: {marker}")
        sys.exit(1)

print(f"Verified lines {start}-{end} contain expected content. Proceeding.")

# Determine indentation used in this block (count leading spaces of the push line)
push_line = None
for l in segment:
    if "chain.blocks.push(block.clone());" in l:
        push_line = l
        break
indent = push_line[:len(push_line) - len(push_line.lstrip(" "))]

new_block_lines = [
    f"{indent}// [P1-FIX 2026-08-16] Per-transaction validation now runs\n",
    f"{indent}// BEFORE this block is appended. Every tx's signature, nonce\n",
    f"{indent}// sequencing, and sender balance is checked against current\n",
    f"{indent}// chain state via validate_incoming_block() (chain.rs). If\n",
    f"{indent}// any transaction fails, the WHOLE block is rejected — an\n",
    f"{indent}// already-mined peer block's tx list is fixed by its\n",
    f"{indent}// hash/PoW, so transactions cannot be selectively dropped\n",
    f"{indent}// the way mine_pending_transactions() drops bad txs from\n",
    f"{indent}// its own mempool.\n",
    f"{indent}if !chain.validate_incoming_block(&block) {{\n",
    f"{indent}    eprintln!(\n",
    f"{indent}        \"[NET] Peer {{}} sent block {{}} that failed transaction validation. Rejecting.\",\n",
    f"{indent}        ip, block.index\n",
    f"{indent}    );\n",
    f"{indent}    return false;\n",
    f"{indent}}}\n",
    f"{indent}\n",
    f"{indent}chain.apply_incoming_block_transactions(&block);\n",
    f"{indent}chain.blocks.push(block.clone());\n",
    f"{indent}chain.save();\n",
    f"{indent}println!(\"[NET] Accepted block {{}} from peer {{}}.\", block.index, ip);\n",
]

new_lines = lines[:start-1] + new_block_lines + lines[end:]

with open(path, "w") as f:
    f.writelines(new_lines)

print("network.rs patched successfully via line-indexed replacement.")
