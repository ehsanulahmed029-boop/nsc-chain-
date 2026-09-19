use crate::validator_registry::ValidatorRegistry;

#[derive(Debug)]
pub struct Committee {

    pub members: Vec<String>,

}

impl Committee {

    pub fn new() -> Self {

        Self {
            members: Vec::new(),
        }

    }

    pub fn build(
        &mut self,
        registry: &ValidatorRegistry,
        size: usize,
    ) {

        let mut validators =
            registry.validators
                .iter()
                .collect::<Vec<_>>();

        validators.sort_by(
            |a, b|
            b.1.stake.cmp(
                &a.1.stake
            )
        );

        self.members.clear();

        for (address, _) in
            validators.iter().take(size)
        {

            self.members.push(
                (*address).clone()
            );

        }
    }

    pub fn is_member(
        &self,
        validator: &str,
    ) -> bool {

        self.members.contains(
            &validator.to_string()
        )

    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== VALIDATOR COMMITTEE ====="
        );

        for member in
            &self.members
        {

            println!(
                "{}",
                member
            );

        }
    }
}
