use crate::peer::Peer;
use std::collections::HashMap;
use crate::seeds::seed_nodes;

#[derive(Debug)]
pub struct PeerManager {
pub peers: Vec<Peer>,
pub reputation: HashMap<String, i32>,
pub banned_peers: Vec<String>,
}

impl PeerManager {
pub fn new() -> Self {
Self {
peers: Vec::new(),
reputation: HashMap::new(),
banned_peers: Vec::new(),
}
}
pub fn reward(
    &mut self,
    peer: &str,
) {
    let score =
        self.reputation
            .entry(peer.to_string())
            .or_insert(0);

    *score += 1;
}
pub fn punish(
    &mut self,
    peer: &str,
) {
    let score =
        self.reputation
            .entry(peer.to_string())
            .or_insert(0);

    *score -= 5;
}
pub fn auto_ban(
    &mut self,
    peer: &str,
) {

    if let Some(score) =
        self.reputation.get(peer)
    {
        if *score < -50 {

            self.ban_peer(
                peer.to_string()
            );
        }
    }
}
pub fn network_health(
    &self,
) {
    println!(
        "\n=== NETWORK HEALTH ==="
    );

    println!(
        "Known Peers: {}",
        self.peer_count()
    );

    if self.peer_count() >= 3 {
        println!(
            "Status: HEALTHY"
        );
    } else {
        println!(
            "Status: LOW PEERS"
        );
    }
}
pub fn load_seed_nodes(
    &mut self,
) {
    println!(
        "\n=== LOADING SEED NODES ==="
    );

    for seed in seed_nodes() {
        self.add_peer(
            seed.to_string()
        );
    }
}
pub fn add_peer(
    &mut self,
    address: String,
) {
    self.peers.push(
        Peer::new(address)
    );
}

pub fn peer_count(
    &self,
) -> usize {
    self.peers.len()
}

pub fn list_peers(
    &self,
) {
    println!("\n=== PEERS ===");

    for peer in &self.peers {
        println!(
            "{} | rep={} | msgs={} | banned={}",
            peer.address,
            peer.reputation,
            peer.messages_sent,
            peer.banned
        );
    }
}

pub fn remove_banned(
    &mut self,
) {
    self.peers.retain(
        |peer| !peer.banned
    );
}

pub fn reputation_report(
    &self,
) {
    println!(
        "\n=== REPUTATION REPORT ==="
    );

    for peer in &self.peers {
        println!(
            "{} => {}",
            peer.address,
            peer.reputation
        );
    }
}

pub fn detect_spam(
    &self,
) {
    for peer in &self.peers {
        if peer.messages_sent > 1000 {
            println!(
                "Spam detected: {}",
                peer.address
            );
        }
    }
}

pub fn discover_peer(
    &mut self,
    address: String,
) {
    self.add_peer(address);
}

pub fn bootstrap(
    &mut self,
) {
    println!(
        "\n=== NETWORK BOOTSTRAP ==="
    );

    self.load_seed_nodes();

    println!(
        "Loaded {} peers",
        self.peer_count()
    );
}

pub fn reward_peer(
    &mut self,
    address: &str,
    points: i32,
) {
    for peer in &mut self.peers {
        if peer.address == address {
            peer.increase_reputation(points);
        }
    }
}

pub fn punish_peer(
    &mut self,
    address: &str,
    points: i32,
) {
    for peer in &mut self.peers {
        if peer.address == address {
            peer.decrease_reputation(points);
        }
    }
}

pub fn show_reputation(
    &self,
) {
    println!(
        "\n=== PEER REPUTATION ==="
    );

    for peer in &self.peers {
        println!(
            "{} => {}",
            peer.address,
            peer.reputation
        );
    }
}

pub fn ban_peer(
    &mut self,
    address: String,
) {
    if !self.banned_peers.contains(
        &address
    ) {
        self.banned_peers.push(
            address.clone()
        );
    }

    for peer in &mut self.peers {
        if peer.address == address {
            peer.banned = true;
        }
    }
}

pub fn list_banned(
    &self,
) {
    println!(
        "\n=== BANNED PEERS ==="
    );

    for peer in &self.banned_peers {
        println!("{}", peer);
    }
}

pub fn get_peer_addresses(
    &self,
) -> Vec<String> {
    self.peers
        .iter()
        .map(
            |p| p.address.clone()
        )
        .collect()
}

}
