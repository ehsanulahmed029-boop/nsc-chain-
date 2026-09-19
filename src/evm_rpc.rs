// ============================================================
// NUSACOIN (NSC) — evm_rpc.rs
// Minimal Ethereum-style JSON-RPC server so MetaMask / Binance
// Wallet can add NSC as a custom network. Runs on its own port
// (8545) and its own tokio runtime — fully additive, does not
// touch the existing tiny_http API on port 8080.
// ============================================================

use jsonrpsee::server::ServerBuilder;
use jsonrpsee::RpcModule;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::chain::Blockchain;
use crate::evm_tx;
use sha3::{Digest, Keccak256};

pub struct EvmState {
    pub blockchain: Arc<Mutex<Blockchain>>,
}

/// NSC's EVM-facing chain ID — defined once in evm_tx.rs so
/// transaction.rs (signature re-verification) and this RPC
/// module share the same value.
use crate::evm_tx::NSC_EVM_CHAIN_ID;

/// Derives a deterministic fake contract address for a custom
/// token from its symbol, so it can be added to wallets like a
/// real ERC-20 token via "Add Custom Token".
pub fn token_contract_address(symbol: &str) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(symbol.as_bytes());
    let hash = hasher.finalize();
    format!("0x{}", hex::encode(&hash[12..32]))
}

/// Minimal ABI encoding for a single dynamic `string` return value.
fn encode_abi_string(s: &str) -> String {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut hex_data = hex::encode(bytes);
    while hex_data.len() % 64 != 0 {
        hex_data.push('0');
    }
    format!("0x{:0>64x}{:0>64x}{}", 32, len, hex_data)
}

pub async fn start_evm_rpc_server(
    chain_height: u64,
    blockchain: Arc<Mutex<Blockchain>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // [PERSISTENCE] Load previously saved EVM balances/nonces so
    // a node restart does not wipe out 0x... account state.
    {
        let (saved_balances, saved_nonces) = crate::storage::load_evm_state();
        let mut chain = blockchain.lock().unwrap();
        for (addr, bal) in saved_balances {
            chain.balances.insert(addr, bal);
        }
        for (addr, nonce) in saved_nonces {
            chain.nonces.insert(addr, nonce);
        }
    }

    let state = Arc::new(EvmState {
        blockchain,
    });
    let mut module = RpcModule::new(state);

    // eth_chainId — MetaMask calls this first when you "Add Network".
    module.register_method("eth_chainId", |_params, _ctx, _ext| {
        format!("0x{:x}", NSC_EVM_CHAIN_ID)
    })?;

    // net_version — legacy chain id call some wallets still use.
    module.register_method("net_version", |_params, _ctx, _ext| {
        NSC_EVM_CHAIN_ID.to_string()
    })?;

    // eth_blockNumber — current chain height, hex-encoded.
    // [FIX] Previously captured chain_height once at server startup via
    // a closure, so it never changed afterwards and broke wallet
    // confirmation tracking. Now reads the live height from the chain
    // on every call, same as eth_getBlockByNumber does.
    module.register_method("eth_blockNumber", |_params, ctx, _ext| {
        let chain = ctx.blockchain.lock().expect("chain lock");
        let height = chain.chain_height();
        format!("0x{:x}", height)
    })?;

    // eth_syncing — report "not syncing" so wallets treat the node as ready.
    module.register_method("eth_syncing", |_params, _ctx, _ext| {
        false
    })?;

    // eth_getBalance — reads directly from the live in-memory
    // balances map. Address is expected as a JSON array:
    // ["0xabc...", "latest"]. Returns hex-encoded wei-style value
    // (we treat 1 NSC == 1 unit, no 18-decimal scaling, for now).
    module.register_method("eth_getBalance", |params, ctx, _ext| {
        let parsed: Vec<serde_json::Value> = params.parse().unwrap_or_default();
        let address = parsed.get(0)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let chain = ctx.blockchain.lock().unwrap();
        let balance = chain.get_balance(&address);
        drop(chain);

        // Balances are now stored natively in 18-decimal (wei-style)
        // u128 units, so no scaling is needed — pass through directly.
        let result = format!("0x{:x}", balance);
        println!("[EVM-RPC] eth_getBalance -> {}", result);
        result
    })?;

    // eth_getTransactionCount — used by wallets as the tx nonce.
    module.register_method("eth_getTransactionCount", |params, ctx, _ext| {
        let parsed: Vec<serde_json::Value> = params.parse().unwrap_or_default();
        let address = parsed.get(0)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let chain = ctx.blockchain.lock().unwrap();
        let nonce = chain.nonces.get(&address).copied().unwrap_or(0);
        drop(chain);

        format!("0x{:x}", nonce)
    })?;

    // eth_gasPrice — flat 1 gwei, gas is not really metered here.
    module.register_method("eth_gasPrice", |_params, _ctx, _ext| {
        "0x3b9aca00".to_string() // 1_000_000_000 wei
    })?;

    // eth_estimateGas — flat 21000, standard transfer cost.
    module.register_method("eth_estimateGas", |_params, _ctx, _ext| {
        "0x5208".to_string() // 21000
    })?;

    // eth_getTransactionReceipt — reports success/failure and
    // basic fields for a tx previously accepted by
    // eth_sendRawTransaction.
    module.register_method("eth_getTransactionReceipt", |params, ctx, _ext| {
        let parsed: Vec<serde_json::Value> = params.parse().unwrap_or_default();
        let tx_hash = parsed.get(0).and_then(|v| v.as_str()).unwrap_or("").to_lowercase();

        let chain = ctx.blockchain.lock().unwrap();
        match chain.evm_receipts.get(&tx_hash) {
            Some(r) => {
                let logs: Vec<serde_json::Value> = r.logs.iter().map(|l| serde_json::json!({
                    "address": l.address,
                    "topics": l.topics,
                    "data": l.data,
                    "blockNumber": format!("0x{:x}", r.block_number),
                    "blockHash": r.block_hash,
                    "transactionHash": r.tx_hash,
                    "transactionIndex": format!("0x{:x}", r.tx_index),
                    "logIndex": format!("0x{:x}", l.log_index),
                    "removed": false,
                })).collect();

                serde_json::json!({
                    "transactionHash": r.tx_hash,
                    "status": if r.status { "0x1" } else { "0x0" },
                    "from": r.from,
                    "to": r.to,
                    "blockNumber": format!("0x{:x}", r.block_number),
                    "blockHash": r.block_hash,
                    "transactionIndex": format!("0x{:x}", r.tx_index),
                    "cumulativeGasUsed": format!("0x{:x}", r.cumulative_gas_used),
                    "gasUsed": format!("0x{:x}", r.gas_used),
                    "logs": logs,
                    "logsBloom": format!("0x{}", "0".repeat(512)),
                    "contractAddress": r.contract_address,
                })
            }
            None => serde_json::Value::Null,
        }
    })?;

    // eth_sendRawTransaction — the core transfer path. Decodes
    // the RLP payload MetaMask/Binance Wallet produced, recovers
    // the sender from the ECDSA signature, then queues the
    // transfer into the shared mempool via chain.transfer_evm()
    // so it goes through the same validation, mining, and block
    // inclusion pipeline as legacy NSC... (Ed25519) transactions.
    module.register_method("eth_sendRawTransaction", |params, ctx, _ext| {
        println!("[EVM-RPC] eth_sendRawTransaction called, raw params: {:?}", params);
        let parsed: Vec<String> = params.parse().unwrap_or_default();
        let raw_hex = parsed.get(0).cloned().unwrap_or_default();
        println!("[EVM-RPC] raw tx hex (len={}): {}", raw_hex.len(), raw_hex);
        let raw_hex_trimmed = raw_hex.trim_start_matches("0x");

        let raw_bytes = match hex::decode(raw_hex_trimmed) {
            Ok(b) => b,
            Err(e) => {
                println!("[EVM-RPC] hex decode FAILED: {}", e);
                return serde_json::json!({"error": format!("invalid hex: {}", e)});
            }
        };

        let mut hasher = Keccak256::new();
        hasher.update(&raw_bytes);
        let eth_tx_hash = format!("0x{}", hex::encode(hasher.finalize()));

        let decoded = match evm_tx::decode_legacy_tx(&raw_bytes) {
            Ok(d) => d,
            Err(e) => {
                println!("[EVM-RPC] RLP decode FAILED: {}", e);
                return serde_json::json!({"error": format!("decode failed: {}", e)});
            }
        };
        println!("[EVM-RPC] decoded tx: nonce={} to={:?} value={} gas_limit={} v={}", decoded.nonce, decoded.to, decoded.value, decoded.gas_limit, decoded.v);

        let sender = match evm_tx::recover_sender(&decoded, NSC_EVM_CHAIN_ID) {
            Ok(s) => s,
            Err(e) => {
                println!("[EVM-RPC] sender recovery FAILED: {}", e);
                return serde_json::json!({"error": format!("signature recovery failed: {}", e)});
            }
        };
        println!("[EVM-RPC] recovered sender: {}", sender);

        let to = decoded.to.map(|b| format!("0x{}", hex::encode(b)));
        // MetaMask sends `value` in wei (18 decimals); this chain now
        // stores NSC amounts natively in the same 18-decimal u128
        // format, so no scaling is needed.
        let amount_nsc: u128 = decoded.value;

        let status = match &to {
            Some(receiver) => {
                let mut chain = ctx.blockchain.lock().unwrap();
                chain.transfer_evm(
                    sender.clone(),
                    receiver.clone(),
                    amount_nsc,
                    raw_hex_trimmed.to_string(),
                    eth_tx_hash.clone(),
                )
            }
            None => false, // contract creation not supported yet
        };

        println!("[EVM-RPC] transfer_evm accepted into mempool: {}", status);

        // [EVM-RECEIPTS] No receipt is written here anymore.
        // eth_getTransactionReceipt only reports a tx once it has
        // actually been mined into a block (see
        // mine_pending_transactions() in chain.rs), with real
        // block_number/block_hash instead of the old fake "0x1".
        serde_json::Value::String(eth_tx_hash)
    })?;

    // eth_maxPriorityFeePerGas — flat tip, no real fee market here.
    module.register_method("eth_maxPriorityFeePerGas", |_params, _ctx, _ext| {
        "0x3b9aca00".to_string() // 1 gwei
    })?;

    // eth_feeHistory — minimal static response so wallets that
    // probe for EIP-1559 fee data don't error out. We report
    // baseFeePerGas as a flat value and no real reward history.
    module.register_method("eth_feeHistory", |params, _ctx, _ext| {
        let parsed: Vec<serde_json::Value> = params.parse().unwrap_or_default();
        let block_count = parsed.get(0).and_then(|v| v.as_u64()).unwrap_or(1).max(1);
        let base_fees: Vec<String> = (0..=block_count).map(|_| "0x3b9aca00".to_string()).collect();
        serde_json::json!({
            "oldestBlock": "0x1",
            "baseFeePerGas": base_fees,
            "gasUsedRatio": vec![0.5f64; block_count as usize],
            "reward": serde_json::Value::Null,
        })
    })?;

    // eth_getBlockByNumber — minimal synthetic block so wallets
    // that fetch "latest" for gas/base-fee context don't error.
    module.register_method("eth_getBlockByNumber", move |_params, ctx, _ext| {
        let chain = ctx.blockchain.lock().unwrap();
        let height = chain.chain_height();
        drop(chain);
        let result = serde_json::json!({
            "number": format!("0x{:x}", height),
            "hash": format!("0x{}", "0".repeat(64)),
            "parentHash": format!("0x{}", "0".repeat(64)),
            "timestamp": format!("0x{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)),
            "gasLimit": "0x1c9c380",
            "gasUsed": "0x0",
            "baseFeePerGas": "0x3b9aca00",
            "transactions": [],
            "miner": "0x0000000000000000000000000000000000000000",
            "difficulty": "0x0",
            "extraData": "0x",
            "size": "0x0",
            "stateRoot": format!("0x{}", "0".repeat(64)),
            "transactionsRoot": format!("0x{}", "0".repeat(64)),
            "receiptsRoot": format!("0x{}", "0".repeat(64)),
            "nonce": "0x0000000000000000",
            "sha3Uncles": format!("0x{}", "0".repeat(64)),
            "uncles": [],
            "logsBloom": format!("0x{}", "0".repeat(512)),
            "mixHash": format!("0x{}", "0".repeat(64)),
            "totalDifficulty": "0x0",
        });
        println!("[EVM-RPC] eth_getBlockByNumber -> {}", result.to_string());
        result
    })?;

    // eth_getCode — no contracts yet, every address is an EOA.
    module.register_method("eth_getCode", |_params, _ctx, _ext| {
        "0x".to_string()
    })?;

    // eth_call — simulates minimal ERC-20 read functions
    // (balanceOf, decimals, totalSupply, symbol, name) for custom
    // tokens created via /token/create. Each token gets a
    // eth_getLogs — naive full-scan implementation. Iterates every
    // stored EvmReceipt, filters by block range / address / topics.
    // Fine at current chain size; revisit with a per-block bloom
    // filter index if evm_receipts grows large enough to make a
    // full scan slow on every call.
    //
    // Filter object fields (all optional, per the standard eth_getLogs
    // JSON-RPC spec): fromBlock, toBlock (hex block number or "latest"),
    // address (single string or array of strings), topics (array,
    // each entry either null, a single topic string, or an array of
    // topic strings treated as OR at that position).
    module.register_method("eth_getLogs", |params, ctx, _ext| {
        let parsed: Vec<serde_json::Value> = params.parse().unwrap_or_default();
        let filter = parsed.get(0).cloned().unwrap_or(serde_json::json!({}));

        let chain = ctx.blockchain.lock().unwrap();
        let latest_height = chain.chain_height();

        let parse_block_tag = |v: Option<&serde_json::Value>, default: u64| -> u64 {
            match v.and_then(|x| x.as_str()) {
                Some("latest") | Some("pending") | None => default,
                Some("earliest") => 0,
                Some(s) => {
                    let trimmed = s.trim_start_matches("0x");
                    u64::from_str_radix(trimmed, 16).unwrap_or(default)
                }
            }
        };

        let from_block = parse_block_tag(filter.get("fromBlock"), 0);
        let to_block = parse_block_tag(filter.get("toBlock"), latest_height as u64);

        // Address filter: single string or array of strings, all
        // lower-cased for case-insensitive comparison.
        let address_filter: Option<Vec<String>> = match filter.get("address") {
            None => None,
            Some(serde_json::Value::String(s)) => Some(vec![s.to_lowercase()]),
            Some(serde_json::Value::Array(arr)) => Some(
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_lowercase())
                    .collect(),
            ),
            Some(_) => None,
        };

        // Topics filter: array of positions, each position is either
        // null (match anything), a single topic string, or an array
        // of topic strings (OR at that position).
        let topics_filter: Vec<Option<Vec<String>>> = filter
            .get("topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|pos| match pos {
                        serde_json::Value::Null => None,
                        serde_json::Value::String(s) => Some(vec![s.to_lowercase()]),
                        serde_json::Value::Array(inner) => Some(
                            inner
                                .iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_lowercase())
                                .collect(),
                        ),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut results: Vec<serde_json::Value> = Vec::new();

        for receipt in chain.evm_receipts.values() {
            if receipt.block_number < from_block || receipt.block_number > to_block {
                continue;
            }

            for log in &receipt.logs {
                if let Some(ref allowed) = address_filter {
                    if !allowed.contains(&log.address.to_lowercase()) {
                        continue;
                    }
                }

                let mut topics_match = true;
                for (i, pos_filter) in topics_filter.iter().enumerate() {
                    if let Some(allowed_topics) = pos_filter {
                        match log.topics.get(i) {
                            Some(t) if allowed_topics.contains(&t.to_lowercase()) => {}
                            _ => {
                                topics_match = false;
                                break;
                            }
                        }
                    }
                }
                if !topics_match {
                    continue;
                }

                results.push(serde_json::json!({
                    "address": log.address,
                    "topics": log.topics,
                    "data": log.data,
                    "blockNumber": format!("0x{:x}", receipt.block_number),
                    "blockHash": receipt.block_hash,
                    "transactionHash": receipt.tx_hash,
                    "transactionIndex": format!("0x{:x}", receipt.tx_index),
                    "logIndex": format!("0x{:x}", log.log_index),
                    "removed": false,
                }));
            }
        }

        serde_json::Value::Array(results)
    })?;

    // deterministic fake contract address derived from its symbol,
    // so wallets like Trust Wallet can be pointed at that address
    // via "Add Custom Token" and read real balances.
    module.register_method("eth_call", |params, _ctx, _ext| {
        let parsed: Vec<serde_json::Value> = params.parse().unwrap_or_default();
        let call_obj = parsed.get(0).cloned().unwrap_or(serde_json::json!({}));
        let to = call_obj.get("to").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let data = call_obj.get("data").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let data_trimmed = data.trim_start_matches("0x");

        if data_trimmed.len() < 8 || to.is_empty() {
            return format!("0x{}", "0".repeat(64));
        }
        let selector = &data_trimmed[0..8];

        let tokens = crate::storage::load_tokens();
        let matched = tokens.iter().find(|t| token_contract_address(&t.symbol) == to);

        let token = match matched {
            Some(t) => t,
            None => return format!("0x{}", "0".repeat(64)),
        };

        match selector {
            "70a08231" => {
                // balanceOf(address)
                if data_trimmed.len() < 8 + 64 {
                    return format!("0x{}", "0".repeat(64));
                }
                let addr_param = &data_trimmed[8+24..8+64];
                let addr = format!("0x{}", addr_param.to_lowercase());
                // [FIX] token.balance_of() already returns an
                // 18-decimal-scaled u128 (Token supply/balances are
                // stored pre-scaled by DECIMALS at /token/create and
                // /token/transfer time, same convention as native NSC
                // and pool reserves). Multiplying by 1e18 again here
                // double-scaled the value, making wallets like MetaMask
                // display balances ~1e18x too large. Return as-is.
                let bal = token.balance_of(&addr);
                format!("0x{:0>64x}", bal)
            },
            "313ce567" => {
                // decimals()
                format!("0x{:0>64x}", 18)
            },
            "18160ddd" => {
                // totalSupply()
                // [FIX] Same double-scaling issue as balanceOf above --
                // token.total_supply is already 18-decimal-scaled.
                format!("0x{:0>64x}", token.total_supply)
            },
            "95d89b41" => {
                // symbol()
                encode_abi_string(&token.symbol)
            },
            "06fdde03" => {
                // name()
                encode_abi_string(&token.name)
            },
            _ => format!("0x{}", "0".repeat(64)),
        }
    })?;

    let addr: SocketAddr = "0.0.0.0:8545".parse()?;
    let server = ServerBuilder::default().build(addr).await?;
    let handle = server.start(module);

    println!("[EVM-RPC] Ethereum-style JSON-RPC listening on 0.0.0.0:8545 (chainId={})", NSC_EVM_CHAIN_ID);

    // Keep the server alive for the lifetime of this future.
    handle.stopped().await;
    Ok(())
}
