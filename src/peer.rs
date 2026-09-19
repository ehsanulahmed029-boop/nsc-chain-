#[derive(Debug, Clone)]
pub struct Peer {
    pub address: String,
    pub reputation: i32,
    pub messages_sent: u64,
    pub banned: bool,
}

impl Peer {
    pub fn new(
    address: String,
) -> Self {

    Self {
        address,
        reputation: 100,
        messages_sent: 0,
        banned: false,
    }
}
pub fn increase_reputation(
    &mut self,
    points: i32,
) {

    self.reputation += points;
}
pub fn decrease_reputation(
    &mut self,
    points: i32,
) {

    self.reputation -= points;

    if self.reputation <= 0 {

        self.banned = true;
    }
}
pub fn record_message(
    &mut self,
) {

    self.messages_sent += 1;

    if self.messages_sent > 1000 {

        self.decrease_reputation(10);
    }
}
pub fn show_status(
    &self,
) {

    println!(
        "\n=== PEER STATUS ==="
    );

    println!(
        "Address: {}",
        self.address
    );

    println!(
        "Reputation: {}",
        self.reputation
    );

    println!(
        "Messages: {}",
        self.messages_sent
    );

    println!(
        "Banned: {}",
        self.banned
    );

}
}
