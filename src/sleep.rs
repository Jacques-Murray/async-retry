// Author: Jacques Murray

//! Provides a runtime-agnostic sleep function.
//!
//! This module uses feature flags (`tokio-timer`, `async-std-timer`)
//! to determine which runtime's sleep function to use.

use std::time::Duration;

/// Sleeps for the specified duration, using the async runtime
/// selected by the crate's feature flags.
///
/// Will produce a compile error if no timer feature is enabled.
pub async fn sleep(duration: Duration) {
    // This is the idiomatic way to handle runtime-agnostic timers.
    cfg_if::cfg_if! {
        if #[cfg(feature = "tokio-timer")] {
            // Use tokio's sleep
            tokio::time::sleep(duration).await;
        } else if #[cfg(feature = "async-std-timer")] {
            // Use async-std's sleep
            async_std::task::sleep(duration).await;
        } else {
            // Compile error if no runtime is selected.
            // This forces the user to make a choice.
            compile_error!("No async timer feature enabled. Please enable 'tokio-timer' or 'async-std-timer'.");
        }
    }
  }