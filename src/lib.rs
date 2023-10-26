#[cfg(feature = "local")]
mod local_deferred_future;
#[cfg(feature = "local")]
pub use local_deferred_future::*;
#[cfg(feature = "thread")]
mod thread_deferred_future;
#[cfg(feature = "thread")]
pub use thread_deferred_future::*;