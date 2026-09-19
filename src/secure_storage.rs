use std::fs;
use crate::encryption;

pub fn save_wallet(
    address: &str,
    private_key: &str,
    password: &str,
) {

    let encrypted =
        encryption::encrypt(
            private_key,
            password,
        );

    let data = format!(
        "{}\n{}",
        address,
        encrypted
    );

    fs::write(
        "wallet.dat",
        data
    )
    .unwrap();
}

pub fn load_wallet()
    -> Option<String>
{
    fs::read_to_string(
        "wallet.dat"
    )
    .ok()
}
