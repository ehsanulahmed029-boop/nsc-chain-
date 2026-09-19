import shutil, sys

CERT_RS = "/root/nsc-chain/src/recovery_snapshot_certificate.rs"
REGISTRY_RS = "/root/nsc-chain/src/recovery_certificate_registry.rs"
EXPIRATION_RS = "/root/nsc-chain/src/recovery_certificate_expiration.rs"

def patch(path, old, new, name):
    shutil.copy2(path, path + ".bak_before_cert_wireup")
    with open(path) as f:
        c = f.read()
    n = c.count(old)
    if n != 1:
        print(f"[ABORT] {name}: found {n} matches (expected 1)")
        sys.exit(1)
    with open(path, "w") as f:
        f.write(c.replace(old, new))
    print(f"[OK] {name}")

patch(CERT_RS,
    "#[derive(Debug, Clone)]\npub struct RecoveryCertificate {",
    "#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub struct RecoveryCertificate {",
    "cert derive")

patch(REGISTRY_RS,
    "pub struct RecoveryCertificateRegistry {\n\n    pub certificates:\n        HashMap<String, RecoveryCertificate>,\n}",
    "#[derive(Clone, serde::Serialize, serde::Deserialize)]\npub struct RecoveryCertificateRegistry {\n\n    pub certificates:\n        HashMap<String, RecoveryCertificate>,\n}",
    "registry derive")

with open(EXPIRATION_RS) as f:
    ec = f.read()
if "UNUSED BY DESIGN" not in ec:
    shutil.copy2(EXPIRATION_RS, EXPIRATION_RS + ".bak_before_cert_wireup")
    notice = ("// WARNING: UNUSED BY DESIGN (2026-08-09). Checkpoint recovery\n"
               "// certificates intentionally never expire. Not wired in.\n"
               "use std::collections::HashMap;")
    ec2 = ec.replace("use std::collections::HashMap;", notice)
    if ec2 == ec:
        print("[ABORT] expiration dead-mark: pattern not found")
        sys.exit(1)
    with open(EXPIRATION_RS, "w") as f:
        f.write(ec2)
    print("[OK] expiration dead-mark")
else:
    print("[SKIP] already marked")

print("[PART1 DONE]")
