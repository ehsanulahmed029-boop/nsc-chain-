use std::collections::HashSet;

pub struct RecoveryAuthorityGovernance {

    authorities:
        HashSet<String>,
}

impl RecoveryAuthorityGovernance {

    pub fn new() -> Self {

        Self {

            authorities:
                HashSet::new(),
        }
    }

    pub fn add_authority(
        &mut self,
        authority: String,
    ) {

        self.authorities.insert(
            authority
        );
    }

    pub fn remove_authority(
        &mut self,
        authority: &str,
    ) {

        self.authorities.remove(
            authority
        );
    }

    pub fn is_authority(
        &self,
        authority: &str,
    ) -> bool {

        self.authorities.contains(
            authority
        )
    }

    pub fn total(
        &self,
    ) -> usize {

        self.authorities.len()
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== RECOVERY AUTHORITIES ====="
        );

        for authority
            in &self.authorities
        {

            println!(
                "{}",
                authority
            );
        }

        println!(
            "Total Authorities: {}",
            self.authorities.len()
        );
    }
}
