use std::collections::HashMap;

#[derive(Debug)]
pub struct JailManager {
    pub jailed: HashMap<String, u64>,
}

impl JailManager {

    pub fn new() -> Self {
        Self {
            jailed: HashMap::new(),
        }
    }

    pub fn jail(
        &mut self,
        validator: String,
        epoch: u64,
    ) {
        self.jailed.insert(
            validator,
            epoch,
        );
    }

    pub fn unjail(
        &mut self,
        validator: &str,
    ) {
        self.jailed.remove(
            validator,
        );
    }

    pub fn is_jailed(
        &self,
        validator: &str,
    ) -> bool {
        self.jailed.contains_key(
            validator,
        )
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== JAILED VALIDATORS ====="
        );

        for (v, epoch)
            in &self.jailed
        {
            println!(
                "{} jailed at epoch {}",
                v,
                epoch
            );
        }
    }
}
