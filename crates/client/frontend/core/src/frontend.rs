//! Trait describing a runnable client front-end.
use anyhow::Result;
use async_trait::async_trait;
use runtime::RuntimeHandle;

/// Frontend abstraction for UI layers.
///
/// Frontends communicate with the game via RuntimeHandle:
/// - Subscribe to events (GameState, Proof, Turn)
/// - Submit player actions
/// - Query current state
///
/// Frontends do NOT own the Runtime - they receive a handle for communication only.
///
/// # Implementations
///
/// - `CliFrontend`: Terminal-based UI (ratatui + crossterm)
/// - Future: `GuiFrontend`, `WebFrontend`, etc.
///
/// # Example Implementation
///
/// ```no_run
/// use async_trait::async_trait;
/// use client_frontend_core::Frontend;
/// use runtime::RuntimeHandle;
/// use anyhow::Result;
///
/// struct MyFrontend;
///
/// #[async_trait]
/// impl Frontend for MyFrontend {
///     async fn run(&mut self, handle: RuntimeHandle) -> Result<()> {
///         // Subscribe to events
///         let mut events = handle.subscribe(runtime::Topic::GameState);
///
///         // Event loop
///         while let Ok(event) = events.recv().await {
///             // Render UI, handle input, etc.
///         }
///
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Frontend: Send {
    /// Run the frontend event loop.
    ///
    /// This method receives a RuntimeHandle for communication with the game.
    /// It should block until the user quits the application.
    ///
    /// # Parameters
    ///
    /// - `handle`: Communication channel to the runtime
    ///
    /// # Errors
    ///
    /// Returns an error if the frontend encounters a fatal error.
    async fn run(&mut self, handle: RuntimeHandle) -> Result<()>;
}

/// Legacy trait for backwards compatibility.
///
/// This trait will be deprecated in favor of `Frontend` which accepts a RuntimeHandle.
#[async_trait]
pub trait FrontendApp: Send {
    async fn run(self) -> Result<()>
    where
        Self: Sized;
}
