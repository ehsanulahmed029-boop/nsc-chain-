use std::collections::HashMap;

#[derive(Debug)]
pub struct PeerManager {
    pub peers: Vec<String>,
    pub banned_peers: Vec<String>,
    pub reputation: HashMap<String, i32>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: Vec::new(),
            banned_peers: Vec::new(),
            reputation: HashMap::new(),
        }
    }

    pub fn add_peer(
        &mut self,
        peer: String,
    ) {

if self.peer_exists(
    &peer
) {
    println!(
        "Peer already exists"
    );

    return;
}
        self.peers.push(
            peer.clone()
        );

        self.reputation.insert(
            peer,
            100,
        );
    }

pub fn peer_count(
    &self,
) -> usize {
    self.peers.len()
} 
pub fn peer_exists(
    &self,
    peer: &str,
) -> bool {
    self.peers.contains(
        &peer.to_string()
    )
}
    pub fn ban_peer(
        &mut self,
        peer: String,
    ) {
        if !self.banned_peers.contains(
            &peer
        ) {
            self.banned_peers.push(
                peer.clone()
            );

            println!(
                "Peer banned: {}",
                peer
            );
        }
    }

    pub fn is_banned(
        &self,
        peer: &str,
    ) -> bool {
        self.banned_peers.contains(
            &peer.to_string()
        )
    }

    pub fn list_banned(
        &self,
    ) {
        println!();
        println!(
            "===== BANNED PEERS ====="
        );

        for peer in
            &self.banned_peers
        {
            println!("{}", peer);
        }
    }

    pub fn reward_peer(
        &mut self,
        peer: &str,
        points: i32,
    ) {
        let rep = self
            .reputation
            .entry(
                peer.to_string()
            )
            .or_insert(100);

        *rep += points;
    }

    pub fn punish_peer(
    &mut self,
    peer: &str,
    points: i32,
) {
    let rep = self
        .reputation
        .entry(
            peer.to_string()
        )
        .or_insert(100);

    *rep -= points;

    if *rep <= 20 {
        self.ban_peer(
            peer.to_string()
        );
    }
}

    pub fn show_reputation(
        &self,
    ) {
        println!();
        println!(
            "===== PEER REPUTATION ====="
        );

        for (peer, score)
            in &self.reputation
        {
            println!(
                "{} => {}",
                peer,
                score
            );
        }
    }

    pub fn list_peers(
        &self,
    ) {
        println!();
        println!("Known Peers:");

        for peer in &self.peers {
            println!("{}", peer);
        }
    }
}
