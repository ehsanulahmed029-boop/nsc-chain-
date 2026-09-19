use std::collections::HashMap;

// ⚠️ DEAD-BY-DESIGN (validator-security audit, 2026-08-11): the only path
// that populates this manager is rotate_validator_key() in main.rs, which
// is never called from anywhere in the codebase. active_keys/key_history
// are therefore permanently empty. show() (still called from the main
// loop) prints the "KEY ROTATION" header with nothing under it, which
// reads as a status confirmation but confirms nothing. No real key
// rotation enforcement currently exists.
#[derive(Debug)]
pub struct KeyRotationManager {
    pub active_keys: HashMap<String, String>,
    pub key_history: HashMap<String, Vec<String>>,
}

impl KeyRotationManager {

    pub fn new() -> Self {
        Self {
            active_keys: HashMap::new(),
            key_history: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        validator: String,
        public_key: String,
    ) {

        self.active_keys.insert(
            validator.clone(),
            public_key.clone(),
        );

        self.key_history
            .entry(validator)
            .or_insert(Vec::new())
            .push(public_key);
    }

    pub fn rotate(
        &mut self,
        validator: &str,
        new_key: String,
    ) {

        self.active_keys.insert(
            validator.to_string(),
            new_key.clone(),
        );

        self.key_history
            .entry(
                validator.to_string()
            )
            .or_insert(Vec::new())
            .push(new_key);
    }

    pub fn active_key(
        &self,
        validator: &str,
    ) -> Option<&String> {

        self.active_keys.get(
            validator
        )
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== KEY ROTATION ====="
        );

        for (validator, key)
            in &self.active_keys
        {

            println!(
                "{} => active key {}",
                validator,
                key
            );
        }
    }

    pub fn show_history(
        &self,
        validator: &str,
    ) {

        println!(
            "\n===== KEY HISTORY ====="
        );

        if let Some(keys) =
            self.key_history.get(
                validator
            )
        {

            for key in keys {

                println!(
                    "{}",
                    key
                );
            }
        }
    }
}
