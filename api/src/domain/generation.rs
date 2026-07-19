#[cfg(test)]
mod tests {
    use super::{GenerationStatus, transition_status};

    #[test]
    fn generation_lifecycle_allows_publishable_and_failure_paths() {
        assert_eq!(
            transition_status(GenerationStatus::Requested, GenerationStatus::Running),
            Ok(GenerationStatus::Running)
        );
        assert_eq!(
            transition_status(GenerationStatus::Running, GenerationStatus::ReviewRequired),
            Ok(GenerationStatus::ReviewRequired)
        );
        assert_eq!(
            transition_status(GenerationStatus::Running, GenerationStatus::Published),
            Ok(GenerationStatus::Published)
        );
        assert_eq!(
            transition_status(GenerationStatus::Running, GenerationStatus::Failed),
            Ok(GenerationStatus::Failed)
        );
    }

    #[test]
    fn generation_lifecycle_rejects_false_publication_and_illegal_retries() {
        assert!(transition_status(GenerationStatus::Failed, GenerationStatus::Published).is_err());
        assert!(transition_status(GenerationStatus::Published, GenerationStatus::Running).is_err());
        assert!(transition_status(GenerationStatus::ReviewRequired, GenerationStatus::Requested).is_err());
    }
}
