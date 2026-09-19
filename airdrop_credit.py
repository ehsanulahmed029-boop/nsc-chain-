import json
import sys
import shutil
from datetime import datetime

STATE_PATH = "/root/nsc-data/evm_state.json"
BATCH_PATH = "/root/nsc-data/airdrop_batch.json"

def main():
    # Backup current state first
    backup_path = STATE_PATH + ".bak_airdrop_" + datetime.now().strftime("%Y%m%d_%H%M%S")
    shutil.copy(STATE_PATH, backup_path)
    print(f"Backed up state to {backup_path}")

    with open(STATE_PATH, "r") as f:
        state = json.load(f)

    with open(BATCH_PATH, "r") as f:
        batch = json.load(f)

    balances = state.get("balances", {})

    credited = 0
    for addr, amount in batch.items():
        addr = addr.lower()
        if not addr.startswith("0x") or len(addr) != 42:
            print(f"SKIP invalid address: {addr}")
            continue
        current = balances.get(addr, 0)
        balances[addr] = current + amount
        print(f"Credited {amount} to {addr} (new balance: {balances[addr]})")
        credited += 1

    state["balances"] = balances

    with open(STATE_PATH, "w") as f:
        json.dump(state, f, indent=2)

    print(f"\nDone. Credited {credited} addresses.")
    print("Restart nsc.service now for changes to take effect.")

if __name__ == "__main__":
    main()
