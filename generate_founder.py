import hashlib

def derive_address(public_key_hex: str) -> str:
    return "NSC" + public_key_hex[:16].upper()

try:
    from nacl.signing import SigningKey
except ImportError:
    print("pip install pynacl --break-system-packages")
    raise SystemExit(1)

sk = SigningKey.generate()
private_key_hex = sk._seed.hex()
public_key_hex = sk.verify_key.encode().hex()
address = derive_address(public_key_hex)

print("====================================")
print("NEW FOUNDER WALLET")
print("====================================")
print(f"Private Key (hex): {private_key_hex}")
print(f"Public Key (hex):  {public_key_hex}")
print(f"Address:           {address}")
print()
print("⚠️ Private Key টা নিরাপদে, অফলাইনে সেভ করে রাখুন।")
print("এটা কখনো হারালে এই wallet-এর কন্ট্রোল হারাবেন।")
