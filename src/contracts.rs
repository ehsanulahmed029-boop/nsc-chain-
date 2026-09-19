#[derive(Debug, Clone)]
pub struct Contract {
    pub name: String,
    pub code: String,
}

#[derive(Debug)]
pub struct ContractVM {
    pub contracts: Vec<Contract>,
}

impl ContractVM {
    pub fn new() -> Self {
        Self {
            contracts: Vec::new(),
        }
    }

    pub fn deploy(
        &mut self,
        contract: Contract,
    ) {
        self.contracts.push(
            contract
        );
    }

    pub fn list(
        &self,
    ) {
        println!(
            "\n===== CONTRACTS ====="
        );

        for c in &self.contracts {
            println!(
                "{}",
                c.name
            );
        }
    }
}

impl Contract {
    pub fn new(
        name: String,
        code: String,
    ) -> Self {
        Self {
            name,
            code,
        }
    }

    pub fn execute(
        &self,
    ) {
        println!(
            "Executing Contract: {}",
            self.name
        );

        println!(
            "{}",
            self.code
        );
    }
}
