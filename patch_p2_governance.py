import sys

path = "/root/nsc-chain/src/main.rs"
with open(path, "r") as f:
    lines = f.readlines()

start = 1790
end   = 1801

segment = "".join(lines[start-1:end])

expected_markers = [
    "pub fn cast_governance_vote(",
    "let _ = governance.weighted_vote(proposal, voter);",
    "Vote recorded for",
]

for marker in expected_markers:
    if marker not in segment:
        print(f"FATAL: expected marker not found in lines {start}-{end}: {marker}")
        sys.exit(1)

print(f"Verified lines {start}-{end} contain expected content. Proceeding.")

new_fn = '''pub fn cast_governance_vote(
    governance: &mut Governance,
    proposal: &str,
    voter: &str,
) {
    // [P2-FIX 2026-08-16] Previously discarded the Result and always
    // printed "Vote recorded" regardless of outcome. Now matches on
    // the real result so operators/logs see the true outcome
    // (AlreadyVoted, ProposalExpired, NotEligibleVoter,
    // EmergencyFrozen, Overflow, etc via GovernanceError's Display).
    match governance.weighted_vote(proposal, voter) {
        Ok(new_total) => {
            println!(
                "[GOV] Vote recorded for '{}' by {}. New tally: {}.",
                proposal, voter, new_total
            );
        }
        Err(e) => {
            eprintln!(
                "[GOV] Vote FAILED for '{}' by {}: {}",
                proposal, voter, e
            );
        }
    }
}
'''

new_lines = lines[:start-1] + [new_fn] + lines[end:]

with open(path, "w") as f:
    f.writelines(new_lines)

print("main.rs patched successfully (cast_governance_vote).")
