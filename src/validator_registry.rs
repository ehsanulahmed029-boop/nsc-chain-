use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    pub stake: u64,
    pub strikes: u32,
    pub jailed: bool,
}

#[derive(Debug, Clone)]
pub struct ValidatorRegistry {
    pub validators: HashMap<String, ValidatorInfo>,
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        address: String,
        stake: u64,
    ) -> bool {

        if stake < 1000 {
            println!(
                "Minimum stake is 1000"
            );

            return false;
        }

        let info = ValidatorInfo {
            stake,
            strikes: 0,
            jailed: false,
        };

        self.validators.insert(
            address,
            info,
        );

        true
    }

    pub fn exists(
        &self,
        address: &str,
    ) -> bool {

        self.validators.contains_key(address)
    }

    pub fn is_active(
        &self,
        address: &str,
    ) -> bool {

        match self.validators.get(address) {
            Some(v) => !v.jailed,
            None => false,
        }
    }

    pub fn validator_count(
        &self,
    ) -> usize {

        self.validators.len()
    }

    pub fn total_stake(
        &self,
    ) -> u64 {

        self.validators
            .values()
            .map(|v| v.stake)
            .sum()
    }

    pub fn deactivate(
        &mut self,
        address: &str,
    ) {

        if let Some(v) =
            self.validators.get_mut(address)
        {
            v.jailed = true;
        }
    }

    pub fn list(
        &self,
    ) {

        println!(
            "\n=== VALIDATOR SET ==="
        );

        for (address, v)
            in &self.validators
        {
            println!(
                "{} | stake={} | jailed={} | strikes={}",
                address,
                v.stake,
                v.jailed,
                v.strikes
            );
        }
    }

    pub fn jail(
        &mut self,
        address: &str,
    ) {

        if let Some(v) =
            self.validators.get_mut(address)
        {
            v.jailed = true;

            println!(
                "Validator jailed: {}",
                address
            );
        }
    }

    pub fn unjail(
        &mut self,
        address: &str,
    ) {

        if let Some(v) =
            self.validators.get_mut(address)
        {
            v.jailed = false;

            println!(
                "Validator unjailed: {}",
                address
            );
        }
    }

    pub fn add_strike(
        &mut self,
        validator: &str,
    ) {

        if let Some(v) =
            self.validators.get_mut(validator)
        {
            v.strikes += 1;

            println!(
                "Strike {} for {}",
                v.strikes,
                validator
            );

            if v.strikes >= 3 {
                v.jailed = true;

                println!(
                    "AUTO JAIL: {}",
                    validator
                );
            }
        }
    }

    pub fn slash(
        &mut self,
        address: &str,
        amount: u64,
    ) {

        if let Some(v) =
            self.validators.get_mut(address)
        {
            if v.stake > amount {
                v.stake -= amount;
            } else {
                v.stake = 0;
            }

            println!(
                "{} slashed by {}",
                address,
                amount
            );
println!(
    "Treasury credit pending: {}",
    amount
);
        }
    }

    pub fn jail_validator(
        &mut self,
        validator: &str,
    ) {

        if let Some(v) =
            self.validators.get_mut(validator)
        {
            v.jailed = true;

            println!(
                "Validator jailed: {}",
                validator
            );
        }
    }
}
