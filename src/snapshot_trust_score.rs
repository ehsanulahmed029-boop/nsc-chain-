#[derive(Debug, Clone)]
pub struct SnapshotProvider {

    pub provider: String,

    pub height: u64,

    pub trust_score: u64,
}

pub struct SnapshotTrustScore;

impl SnapshotTrustScore {

    pub fn select_best(
        providers:
            &[SnapshotProvider],
    ) -> Option<SnapshotProvider> {

        let mut best:
            Option<SnapshotProvider>
                = None;

        for provider
            in providers
        {

            match &best {

                Some(current) => {

                    if provider.trust_score
                        > current.trust_score
                    {
                        best =
                            Some(
                                provider.clone()
                            );
                    }
                }

                None => {

                    best =
                        Some(
                            provider.clone()
                        );
                }
            }
        }

        best
    }

    pub fn show(
        provider:
            &SnapshotProvider,
    ) {

        println!(
            "\n===== SNAPSHOT TRUST SCORE ====="
        );

        println!(
            "Provider: {}",
            provider.provider
        );

        println!(
            "Height: {}",
            provider.height
        );

        println!(
            "Trust Score: {}",
            provider.trust_score
        );
    }
}
