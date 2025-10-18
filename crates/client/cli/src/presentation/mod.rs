//! Terminal presentation components used by the CLI client.
pub mod event_consumer;
pub mod event_loop;
pub mod terminal;
pub mod ui;

pub use event_consumer::CliEventConsumer;
pub use event_loop::EventLoop;
