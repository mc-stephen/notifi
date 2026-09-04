use async_trait::async_trait;

/// Result of testing a provider connection.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub success: bool,
    pub message: String,
}

/// Port for testing provider connections.
#[async_trait]
pub trait ProviderTester: Send + Sync {
    /// Test a provider connection with the given config.
    async fn test(&self, channel_id: &str, provider_id: &str, config: &serde_json::Value) -> TestResult;
}
