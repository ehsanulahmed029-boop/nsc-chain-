import shutil, datetime, sys

path = "/var/www/nsc/dex.html"
backup = f"/root/backups/dex.html.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# 1) Add a new signing function next to signLiquidityAddMessage
old1 = '''async function signLiquidityAddMessage(symbol, tokenAmountScaled, usdtAmount, wallet, timestamp){
  const message = `NSC_LIQUIDITY_ADD:${symbol}:${tokenAmountScaled}:${usdtAmount}:${wallet}:${timestamp}`;
  if(BROWSER_WALLET_PK){
    const signer = new ethers.Wallet(BROWSER_WALLET_PK);
    return await signer.signMessage(message);
  }
  return await window.ethereum.request({
    method: 'personal_sign',
    params: [message, wallet]
  });
}'''

new1 = old1 + '''

async function signLiquidityAddMainMessage(nscAmountStr, usdtAmountStr, wallet, timestamp){
  const message = `NSC_LIQUIDITY_ADD_MAIN:${nscAmountStr}:${usdtAmountStr}:${wallet}:${timestamp}`;
  if(BROWSER_WALLET_PK){
    const signer = new ethers.Wallet(BROWSER_WALLET_PK);
    return await signer.signMessage(message);
  }
  return await window.ethereum.request({
    method: 'personal_sign',
    params: [message, wallet]
  });
}'''

if content.count(old1) != 1:
    print(f"ERROR: signing-function anchor found {content.count(old1)} times, expected 1")
    sys.exit(1)
content = content.replace(old1, new1)
print("Patched: added signLiquidityAddMainMessage()")

# 2) Update addLiq() NSC branch to sign and send wallet/signature/timestamp
old2 = '''if(sym==='NSC'){
  r=await fetch(API+'/liquidity/add',{
  method:'POST',
  headers:{"X-Api-Key":APIKEY,"Content-Type":"application/json"},
  body:JSON.stringify({nsc_amount:tokAmt,usdt_amount:usdt})
});
} else {'''

new2 = '''if(sym==='NSC'){
  const liqMainTimestamp = Math.floor(Date.now()/1000);
  const nscAmountStr = tokAmt.toString();
  const usdtAmountStr = usdt.toString();
  const liqMainSignature = await signLiquidityAddMainMessage(nscAmountStr, usdtAmountStr, EVM_WALLET.addr, liqMainTimestamp);
  r=await fetch(API+'/liquidity/add',{
  method:'POST',
  headers:{"X-Api-Key":APIKEY,"Content-Type":"application/json"},
  body:JSON.stringify({nsc_amount:nscAmountStr,usdt_amount:usdtAmountStr,wallet:EVM_WALLET.addr,signature:liqMainSignature,timestamp:liqMainTimestamp})
});
} else {'''

if content.count(old2) != 1:
    print(f"ERROR: addLiq anchor found {content.count(old2)} times, expected 1")
    sys.exit(1)
content = content.replace(old2, new2)
print("Patched: addLiq() NSC branch now signs the request")

with open(path, "w") as f:
    f.write(content)
