// ============================================================
// NUSACOIN (NSC) — network.rs — Hardened Replacement (DRAFT)
// ============================================================
// THIS FILE IS A DRAFT. IT HAS NOT BEEN COMPILED, RUN, OR
// TESTED AGAINST A SECOND NODE. Treat it as a much-improved
// starting point, not a finished, trustworthy P2P layer.
//
// WHAT THIS FIXES VS THE PREVIOUS VERSION:
// [FIX-01] Real length-prefixed message framing instead of a
//          single 1024-byte read. Messages are read fully
//          before being parsed, with a hard max-size cap.
// [FIX-02] Received transactions are actually parsed and
//          pushed through Mempool::add_transaction() — which
//          re-verifies signature, nonce freshness, etc. — not
//          just printed.
// [FIX-03] Received blocks are actually parsed and validated
//          against the real chain before being accepted —
//          not just printed and silently "propagated."
// [FIX-04] Per-peer reputation (peer.rs's Peer struct) is now
//          actually wired in: malformed or invalid messages
//          decrease reputation; banned peers are disconnected
//          and refused future connections.
// [FIX-05] Connection counter actually decrements on
//          disconnect (previous version could only ever count
//          up to 100 total connections for the life of the
//          process, then refuse everyone forever).
// [FIX-06] Per-IP connection rate limiting, separate from the
//          global connection cap.
// [FIX-07] No unwrap() on socket writes — a peer disconnecting
//          mid-send no longer panics the sending thread.
// [FIX-08] All inbound data is bounded (max message size) and
//          read with a timeout, so a slow/malicious peer can't
//          tie up a thread indefinitely.
// ============================================================

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::block::Block;
use crate::block_message::BlockMessage;
use crate::chain::Blockchain;
use crate::peer::Peer;
use crate::transaction::Transaction;

// ── Constants ────────────────────────────────────────────────

/// Maximum size of any single message, in bytes. Anything
/// claiming to be larger is rejected before we allocate a
/// buffer for it — prevents memory-exhaustion from a hostile
/// or buggy peer.
const MAX_MESSAGE_BYTES: u32 = 8 * 1024 * 1024; // 8 MB

/// Maximum total simultaneous connections across all peers.
const MAX_TOTAL_CONNECTIONS: usize = 200;

/// Maximum simultaneous connections from a single IP.
const MAX_CONNECTIONS_PER_IP: usize = 5;

/// Socket read/write timeout. A peer that doesn't send a full
/// message within this window gets disconnected instead of
/// holding a thread open forever.
const SOCKET_TIMEOUT: Duration = Duration::from_secs(15);

/// Reputation point cost for sending a malformed message.
const PENALTY_MALFORMED: i32 = 15;

/// Reputation point cost for sending a message that parses but
/// fails real validation (bad signature, invalid block, etc).
/// Higher than malformed, because this means the peer is either
/// running broken software or actively trying something.
const PENALTY_INVALID: i32 = 30;

/// Reputation point reward for a message that was fully valid
/// and accepted.
const REWARD_VALID: i32 = 1;

// ── Wire format ──────────────────────────────────────────────
//
// Every message on the wire is:
//   [4 bytes: big-endian u32 length prefix]
//   [N bytes: UTF-8 payload, "TYPE:body" format]
//
// This replaces the old "read up to 1024 bytes and hope it's
// the whole message" approach, which silently truncated any
// real block or chain payload over 1KB.

#[derive(Debug, Clone)]
enum WireMessage {
    Sync,
    Tx(String),
    Block(String),
    Mempool(String),
    PeerAnnounce(String),
    Chain(String),
}

#[derive(Debug)]
enum FrameError {
    TooLarge,
    Io,
    Timeout,
    Empty,
    Malformed,
}

/// Reads exactly one framed message from `stream`, enforcing
/// the size cap. Returns the raw payload string.
///
/// [FIX-01] [FIX-08] Real framing with a size cap and timeout,
/// instead of a single best-effort 1024-byte read.
fn read_framed_message(stream: &mut TcpStream) -> Result<String, FrameError> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
            FrameError::Timeout
        } else {
            FrameError::Io
        }
    })?;

    let len = u32::from_be_bytes(len_buf);

    if len == 0 {
        return Err(FrameError::Empty);
    }
    if len > MAX_MESSAGE_BYTES {
        eprintln!(
            "[NET] Peer claims message size {} bytes, exceeds max {}. Rejecting.",
            len, MAX_MESSAGE_BYTES
        );
        return Err(FrameError::TooLarge);
    }

    let mut payload = vec![0u8; len as usize];
    stream.read_exact(&mut payload).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
            FrameError::Timeout
        } else {
            FrameError::Io
        }
    })?;

    String::from_utf8(payload).map_err(|_| FrameError::Malformed)
}

/// Writes exactly one framed message to `stream`.
///
/// [FIX-07] Returns Result instead of unwrap()-panicking on a
/// write failure (e.g. peer disconnected mid-send).
fn write_framed_message(stream: &mut TcpStream, payload: &str) -> std::io::Result<()> {
    let bytes = payload.as_bytes();
    let len = bytes.len() as u32;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(bytes)?;
    stream.flush()?;
    Ok(())
}

/// Parses a raw payload string into a WireMessage.
fn parse_message(raw: &str) -> Result<WireMessage, FrameError> {
    if raw.trim().is_empty() {
        return Err(FrameError::Empty);
    }

    if raw == "SYNC" {
        return Ok(WireMessage::Sync);
    }
    if let Some(body) = raw.strip_prefix("TX:") {
        return Ok(WireMessage::Tx(body.to_string()));
    }
    if let Some(body) = raw.strip_prefix("BLOCK:") {
        return Ok(WireMessage::Block(body.to_string()));
    }
    if let Some(body) = raw.strip_prefix("MEMPOOL:") {
        return Ok(WireMessage::Mempool(body.to_string()));
    }
    if let Some(body) = raw.strip_prefix("PEER:") {
        return Ok(WireMessage::PeerAnnounce(body.to_string()));
    }
    if let Some(body) = raw.strip_prefix("CHAIN:") {
        return Ok(WireMessage::Chain(body.to_string()));
    }

    Err(FrameError::Malformed)
}

// ── Connection tracking ──────────────────────────────────────

/// Tracks live connection counts so we can enforce both a
/// global cap and a per-IP cap, and so the global cap actually
/// goes back down when connections close.
///
/// [FIX-05] Connections are decremented on disconnect — the
/// previous version only ever incremented, so the node would
/// permanently refuse all new connections after 100 total
/// connections across its entire lifetime.
#[derive(Default)]
pub(crate) struct ConnectionTracker {
    total: usize,
    per_ip: HashMap<String, usize>,
}

impl ConnectionTracker {
    /// Attempts to register a new connection from `ip`.
    /// Returns false (and changes nothing) if either cap would
    /// be exceeded.
    fn try_register(&mut self, ip: &str) -> bool {
        if self.total >= MAX_TOTAL_CONNECTIONS {
            return false;
        }
        let count = self.per_ip.entry(ip.to_string()).or_insert(0);
        if *count >= MAX_CONNECTIONS_PER_IP {
            return false;
        }
        *count += 1;
        self.total += 1;
        true
    }

    /// Must be called exactly once for every successful
    /// try_register(), when that connection closes.
    fn release(&mut self, ip: &str) {
        if let Some(count) = self.per_ip.get_mut(ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.per_ip.remove(ip);
            }
        }
        self.total = self.total.saturating_sub(1);
    }
}

// ── Node ─────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Node {
    pub address: String,
    pub connections: Arc<Mutex<ConnectionTracker>>,
    pub peers: Arc<Mutex<Vec<String>>>,
    /// [FIX-04] Per-address peer reputation, actually wired in
    /// and consulted on every inbound message.
    pub peer_reputation: Arc<Mutex<HashMap<String, Peer>>>,
    /// Shared handle to the real chain and mempool, so inbound
    /// transactions and blocks can actually be validated and
    /// applied rather than just printed.
    pub blockchain: Arc<Mutex<Blockchain>>,
}

impl Node {
    pub fn new(address: String, blockchain: Arc<Mutex<Blockchain>>) -> Self {
        Self {
            address,
            connections: Arc::new(Mutex::new(ConnectionTracker::default())),
            peers: Arc::new(Mutex::new(Vec::new())),
            peer_reputation: Arc::new(Mutex::new(HashMap::new())),
            blockchain,
        }
    }

    pub fn add_peer(&self, peer: String) {
        let mut peers = self.peers.lock().expect("peers lock");
        if !peers.contains(&peer) {
            peers.push(peer.clone());
            println!("[NET] Peer added: {}", peer);
        }
    }

    pub fn list_peers(&self) {
        let peers = self.peers.lock().expect("peers lock");
        println!("\n=== PEERS ===");
        for peer in peers.iter() {
            println!("{}", peer);
        }
    }

    pub fn network_status(&self) {
        let conn = self.connections.lock().expect("connections lock");
        println!("\n=== NETWORK STATUS ===");
        println!("Node Address      : {}", self.address);
        println!("Total Connections  : {}", conn.total);
        println!("Known Peers        : {}", self.peers.lock().expect("peers lock").len());
    }

    pub fn show_connections(&self) {
        let conn = self.connections.lock().expect("connections lock");
        println!("Active Connections: {}", conn.total);
    }

    // ── Outbound ─────────────────────────────────────────────

    /// Sends a single framed message to `peer`. Logs failures
    /// rather than panicking.
    ///
    /// [FIX-07] No unwrap() on the write path.
    pub fn send(&self, peer: &str, message: &str) {
        match TcpStream::connect(peer) {
            Ok(mut stream) => {
                let _ = stream.set_write_timeout(Some(SOCKET_TIMEOUT));
                if let Err(e) = write_framed_message(&mut stream, message) {
                    eprintln!("[NET] Failed to send to {}: {}", peer, e);
                } else {
                    println!("[NET] Sent to {} ({} bytes).", peer, message.len());
                }
            }
            Err(e) => {
                eprintln!("[NET] Cannot connect to {}: {}", peer, e);
            }
        }
    }

    pub fn broadcast_to_peers(&self, peers: &[String], message: &str) {
        for peer in peers {
            self.send(peer, message);
        }
    }

    pub fn announce_self(&self) {
        let peers = self.peers.lock().expect("peers lock").clone();
        let msg = format!("PEER:{}", self.address);
        for peer in peers {
            self.send(&peer, &msg);
        }
    }

    pub fn broadcast_peer(&self, peer: &str, discovered: &str) {
        self.send(peer, &format!("PEER:{}", discovered));
    }

    pub fn request_sync(&self, peer: &str) {
        self.send(peer, "SYNC");
    }

    pub fn send_chain(&self, peer: &str, chain_data: &str) {
        self.send(peer, &format!("CHAIN:{}", chain_data));
    }

    pub fn broadcast_mempool(&self, peer: &str, txs: &str) {
        self.send(peer, &format!("MEMPOOL:{}", txs));
    }

    /// Broadcasts a transaction to a single peer.
    ///
    /// Note: this takes already-serialized tx_data. The caller
    /// is responsible for ensuring tx_data is the canonical
    /// serialized form of a Transaction that the receiving node
    /// can deserialize — typically `serde_json::to_string(&tx)`.
    pub fn broadcast_transaction(&self, peer: &str, tx_data: &str) {
        self.send(peer, &format!("TX:{}", tx_data));
    }

    pub fn broadcast_block_to_peer(&self, peer: &str, block_data: &str) {
        self.send(peer, &format!("BLOCK:{}", block_data));
    }

    pub fn broadcast_block(&self, msg: &BlockMessage) {
        let peers = self.peers.lock().expect("peers lock").clone();
        let block_data = match serde_json::to_string(&msg.block) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("[NET] Failed to serialize block for broadcast: {}", e);
                return;
            }
        };
        println!(
            "[NET] Broadcasting block {} from {} to {} peer(s).",
            msg.block.index,
            msg.sender,
            peers.len()
        );
        self.broadcast_to_peers(&peers, &format!("BLOCK:{}", block_data));
    }

    // ── Reputation helpers ───────────────────────────────────

    /// [FIX-04] Looks up or creates reputation tracking for a peer.
    /// Returns true if the peer is now banned as a result of this
    /// outcome.
    fn record_outcome(&self, ip: &str, delta: i32) -> bool {
        let mut rep = self.peer_reputation.lock().expect("reputation lock");
        let peer = rep
            .entry(ip.to_string())
            .or_insert_with(|| Peer::new(ip.to_string()));

        if delta >= 0 {
            peer.increase_reputation(delta);
        } else {
            peer.decrease_reputation(-delta);
        }
        peer.record_message();

        if peer.banned {
            eprintln!("[NET] Peer {} is now banned (reputation={}).", ip, peer.reputation);
        }

        peer.banned
    }

    fn is_banned(&self, ip: &str) -> bool {
        let rep = self.peer_reputation.lock().expect("reputation lock");
        rep.get(ip).map(|p| p.banned).unwrap_or(false)
    }

    // ── Inbound: listener ────────────────────────────────────

    /// Starts listening for inbound connections and dispatches
    /// each to a handler thread.
    ///
    /// [FIX-05] [FIX-06] Real connection accounting with
    /// per-IP and global caps that actually release on
    /// disconnect, instead of a counter that only ever goes up.
    pub fn start(&self) {
        let listener = match TcpListener::bind(&self.address) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[NET] FATAL: failed to bind {}: {}", self.address, e);
                std::process::exit(1);
            }
        };

        println!("[NET] NSC node listening on {}", self.address);

        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[NET] Connection error: {}", e);
                    continue;
                }
            };

            let ip = stream
                .peer_addr()
                .map(|a| a.ip().to_string())
                .unwrap_or_else(|_| "unknown".to_string());

            if self.is_banned(&ip) {
                eprintln!("[NET] Rejected connection from banned peer: {}", ip);
                // Dropping `stream` here closes the socket.
                continue;
            }

            let registered = {
                let mut conn = self.connections.lock().expect("connections lock");
                conn.try_register(&ip)
            };

            if !registered {
                eprintln!(
                    "[NET] Connection limit reached, rejecting {} (global or per-IP cap).",
                    ip
                );
                continue;
            }

            let node = self.clone();
            let ip_for_thread = ip.clone();

            thread::spawn(move || {
                node.handle_connection(stream, &ip_for_thread);

                // [FIX-05] Always release the slot when the
                // connection ends, regardless of how it ended.
                let mut conn = node.connections.lock().expect("connections lock");
                conn.release(&ip_for_thread);
            });
        }
    }

    /// Handles a single inbound connection for as long as the
    /// peer keeps sending valid frames, up to reputation limits.
    fn handle_connection(&self, mut stream: TcpStream, ip: &str) {
        let _ = stream.set_read_timeout(Some(SOCKET_TIMEOUT));
        let _ = stream.set_write_timeout(Some(SOCKET_TIMEOUT));

        loop {
            if self.is_banned(ip) {
                eprintln!("[NET] Disconnecting now-banned peer: {}", ip);
                return;
            }

            let raw = match read_framed_message(&mut stream) {
                Ok(r) => r,
                Err(FrameError::Timeout) => {
                    // Idle connection — close quietly, not a penalty.
                    return;
                }
                Err(FrameError::TooLarge) => {
                    eprintln!("[NET] Peer {} sent oversized message. Penalizing and closing.", ip);
                    self.record_outcome(ip, -PENALTY_MALFORMED);
                    return; // oversized message also means we can't safely keep reading this stream
                }
                Err(FrameError::Empty) | Err(FrameError::Malformed) | Err(FrameError::Io) => {
                    eprintln!("[NET] Peer {} sent unreadable data. Closing connection.", ip);
                    return;
                }
            };

            let message = match parse_message(&raw) {
                Ok(m) => m,
                Err(_) => {
                    eprintln!("[NET] Peer {} sent malformed message. Penalizing.", ip);
                    if self.record_outcome(ip, -PENALTY_MALFORMED) {
                        return;
                    }
                    continue;
                }
            };

            let valid = self.dispatch_message(message, ip);
            let banned = self.record_outcome(ip, if valid { REWARD_VALID } else { -PENALTY_INVALID });
            if banned {
                return;
            }
        }
    }

    /// Validates and applies a single inbound message.
    ///
    /// Returns true if the message was valid and accepted/acted
    /// on, false if it was well-formed but failed real
    /// validation (bad signature, invalid block, etc). The
    /// caller uses this to adjust peer reputation.
    ///
    /// [FIX-02] [FIX-03] This is the core fix: messages are
    /// actually parsed into real Transaction/Block values and
    /// run through the existing validation/mempool/chain logic
    /// instead of just being printed.
    fn dispatch_message(&self, message: WireMessage, ip: &str) -> bool {
        match message {
            WireMessage::Sync => {
                println!("[NET] Peer {} requested sync.", ip);
                // Real sync response (sending our chain back) should be
                // wired up by the caller, e.g. by calling send_chain()
                // with a serialized chain after this returns. Left as
                // a hook rather than guessed at here, since chain
                // export format/size limits are a deliberate decision,
                // not a network-framing one.
                true
            }

            WireMessage::Tx(tx_json) => {
                let tx: Transaction = match serde_json::from_str(&tx_json) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("[NET] Peer {} sent unparseable transaction: {}", ip, e);
                        return false;
                    }
                };

                // [FIX-02] Real validation, not a println.
                if !tx.verify() {
                    eprintln!(
                        "[NET] Peer {} sent a transaction that failed verification (hash={}).",
                        ip, tx.tx_hash
                    );
                return false;
                }

                let mut chain = self.blockchain.lock().expect("chain lock");

                if chain.is_processed(&tx.tx_hash) {
                    // Already confirmed on-chain — not invalid, just
                    // stale. Don't penalize for this; it's normal
                    // gossip overlap, not misbehavior.
                    println!("[NET] Ignoring already-processed tx {} from {}.", tx.tx_hash, ip);
                    return true;
                }

                if chain.is_blacklisted(&tx.sender) {
                    eprintln!("[NET] Peer {} relayed tx from blacklisted sender.", ip);
                    return false;
                }

                chain.mempool.add_transaction(tx.clone());
                println!("[NET] Accepted transaction {} from peer {}.", tx.tx_hash, ip);

                // Re-gossip to our own peers (best-effort; we only
                // have the connecting IP, not necessarily the peer's
                // listen address — wire this to your real peer-address
                // bookkeeping if it differs from connection IP).
                let peers = self.peers.lock().expect("peers lock").clone();
                drop(chain); // release lock before making outbound calls
                for p in peers {
                    self.broadcast_transaction(&p, &tx_json);
                }

                true
            }

            WireMessage::Block(block_json) => {
                let block: Block = match serde_json::from_str(&block_json) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("[NET] Peer {} sent unparseable block: {}", ip, e);
                        return false;
                    }
                };

                if !block.is_hash_valid() {
                    eprintln!(
                        "[NET] Peer {} sent block {} with invalid hash. Rejecting.",
                        ip, block.index
                    );
                    return false;
                }

                let mut chain = self.blockchain.lock().expect("chain lock");

                if !block.meets_difficulty(chain.difficulty) {
                    eprintln!(
                        "[NET] Peer {} sent block {} that doesn't meet PoW difficulty. Rejecting.",
                        ip, block.index
                    );
                    return false;
                }

                let expected_next_index = chain.blocks.last().map(|b| b.index + 1).unwrap_or(0);

                if block.index == expected_next_index {
                    let last_hash = chain
                        .blocks
                        .last()
                        .map(|b| b.hash.clone())
                        .unwrap_or_else(|| "0".to_string());

                    if block.previous_hash != last_hash {
                        eprintln!(
                            "[NET] Peer {} sent block {} with wrong previous_hash. Rejecting.",
                            ip, block.index
                        );
                        return false;
                    }

                    // [P1-FIX 2026-08-16] Per-transaction validation now runs
                    // BEFORE this block is appended. Every tx's signature, nonce
                    // sequencing, and sender balance is checked against current
                    // chain state via validate_incoming_block() (chain.rs). If
                    // any transaction fails, the WHOLE block is rejected — an
                    // already-mined peer block's tx list is fixed by its
                    // hash/PoW, so transactions cannot be selectively dropped
                    // the way mine_pending_transactions() drops bad txs from
                    // its own mempool.
                    if !chain.validate_incoming_block(&block) {
                        eprintln!(
                            "[NET] Peer {} sent block {} that failed transaction validation. Rejecting.",
                            ip, block.index
                        );
                        return false;
                    }
                    
                    chain.apply_incoming_block_transactions(&block);
                    chain.blocks.push(block.clone());
                    chain.save();
                    println!("[NET] Accepted block {} from peer {}.", block.index, ip);
                } else if block.index > expected_next_index {
                    // We're behind. Don't blindly accept an
                    // out-of-order block — request a real sync
                    // instead of trying to splice it in.
                    println!(
                        "[NET] Peer {} is ahead (block {} vs our {}). Requesting sync.",
                        ip, block.index, expected_next_index
                    );
                    drop(chain);
                    self.request_sync(ip);
                } else {
                    // Older block than what we have — likely a fork
                    // candidate. Hand to fork-handling rather than
                    // silently dropping it.
                    println!(
                        "[NET] Peer {} sent older block {} — treating as fork candidate.",
                        ip, block.index
                    );
                    chain.add_fork(vec![block.clone()]);
                }

                true
            }

            WireMessage::Mempool(_txs) => {
                // Intentionally not auto-trusting bulk mempool dumps.
                // A real implementation should parse this into a
                // Vec<Transaction> and feed each one through the same
                // tx.verify() + mempool.add_transaction() path as
                // WireMessage::Tx above — never insert mempool data
                // directly without per-transaction verification.
                println!(
                    "[NET] Received mempool sync from {} (not yet auto-applied — wire to per-tx verify before enabling).",
                    ip
                );
                true
            }

            WireMessage::PeerAnnounce(peer_addr) => {
                if peer_addr.trim().is_empty() || peer_addr.len() > 256 {
                    return false;
                }
                self.add_peer(peer_addr);
                true
            }

            WireMessage::Chain(_chain_data) => {
                // Intentionally not auto-replacing our chain from an
                // unsolicited CHAIN message. A real implementation
                // should parse this into Vec<Block>, run it through
                // Blockchain::validate_external_chain(), and only then
                // hand it to add_fork() / select_best_chain() for
                // proper fork-choice — never swap chains directly on
                // receipt of unauthenticated peer data.
                println!(
                    "[NET] Received chain sync from {} (not yet auto-applied — wire to validate_external_chain before enabling).",
                    ip
                );
                true
            }
        }
    }
}

