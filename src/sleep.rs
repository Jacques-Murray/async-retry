// Author: Jacques Murray

//! Runtime-agnostic sleep functionality.
//!
//! This module provides a single `sleep` function that works with different async
//! runtimes. The actual implementation is selected at compile time based on feature
//! flags.
//!
//! # Feature Flags
//!
//! You must enable exactly one of these features:
//! - `tokio-timer` - Use Tokio's timer implementation
//! - `async-std-timer` - Use async-std's timer implementation
//!
//! If no timer feature is enabled, you'll get a compile error with a helpful message.
//!
//! # Design
//!
//! This approach using `cfg_if!` is the idiomatic way to provide runtime-agnostic
//! functionality in Rust async libraries. It has zero runtime cost - the compiler
//! selects the correct implementation at build time.

use std::time::Duration;

/// Asynchronously sleeps for the specified duration.
///
/// This function delegates to the appropriate runtime's sleep implementation
/// based on which feature flag is enabled.
///
/// # Arguments
///
/// * `duration` - How long to sleep
///
/// # Compile-Time Behavior
///
/// - With `tokio-timer`: Uses [`tokio::time::sleep`]
/// - With `async-std-timer`: Uses [`async_std::task::sleep`]
/// - With neither: Produces a compile error
///
/// # Examples
///
/// ```rust,no_run
/// use std::time::Duration;
/// # use async_retry::backoff::FixedDelay;
///
/// # async fn example() {
/// // Sleep is called internally by the retry logic
/// // You don't typically call it directly
/// # }
/// ```
pub async fn sleep(duration: Duration) {
    // Use cfg_if for clean compile-time feature selection
    cfg_if::cfg_if! {
        if #[cfg(feature = "tokio-timer")] {
            tokio::time::sleep(duration).await;
        } else if #[cfg(feature = "async-std-timer")] {
            async_std::task::sleep(duration).await;
        } else {
            // Provide a helpful compile error if no timer feature is enabled
            compile_error!(
                "No async timer feature enabled. \
                 Please enable either 'tokio-timer' or 'async-std-timer' in your Cargo.toml."
            );
        }
    }
}