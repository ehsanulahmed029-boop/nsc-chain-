import shutil, sys

REGISTRY_RS = "/root/nsc-chain/src/recovery_certificate_registry.rs"

with open(REGISTRY_RS) as f:
    c = f.read()
shutil.copy2(REGISTRY_RS, REGISTRY_RS + ".bak_before_cert_wireup_p5")

old = "#[derive(Clone, serde::Serialize, serde::Deserialize)]\npub struct RecoveryCertificateRegistry {"
new = "#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]\npub struct RecoveryCertificateRegistry {"

n = c.count(old)
if n != 1:
    print(f"[ABORT] found {n} matches (expected 1)")
    sys.exit(1)

with open(REGISTRY_RS, "w") as f:
    f.write(c.replace(old, new))

print("[OK] added Debug + Default derives")
print("[PART5 DONE]")
