use sha2::{Digest, Sha256};

pub fn encrypt(
    data: &str,
    password: &str,
) -> String {

    let mut hasher = Sha256::new();

    hasher.update(
        format!(
            "{}{}",
            password,
            data
        )
    );

    hex::encode(
        hasher.finalize()
    )
}
pub fn decrypt(
    data: &str,
    _password: &str,
) -> String {

    data.to_string()
}
