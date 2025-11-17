//! Trait describing a runnable client front-end.
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait FrontendApp: Send {
    async fn run(self) -> Result<()>
    where
        Self: Sized;
}
