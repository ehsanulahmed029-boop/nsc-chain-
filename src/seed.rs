#[derive(Debug, Clone)]
pub struct SeedPhrase {
    pub words: Vec<String>,
}

impl SeedPhrase {
    pub fn generate() -> Self {
        Self {
            words: vec![
                "apple".to_string(),
                "moon".to_string(),
                "river".to_string(),
                "stone".to_string(),
                "dragon".to_string(),
                "tree".to_string(),
                "cloud".to_string(),
                "gold".to_string(),
                "ocean".to_string(),
                "star".to_string(),
                "light".to_string(),
                "fire".to_string(),
            ],
        }
    }

    pub fn phrase(&self) -> String {
        self.words.join(" ")
    }
}
