// Author: Jacques Murray

//! # async-retry
//!
//! A library to simplify retrying asynchronous operations with customizable
//! backoff strategies, inspired by the PRD.
//!
//! ## Goals
//!
//! * Provide a simple, ergonomic API for retrying `async` operations.
//! * Offer flexible backoff strategies (Fixed, Exponential, Fibonacci).
//! * Allow conditional retries based on the returned error.
//! * Be runtime-agnostic (supports Tokio and async-std via feature flags).
//!
//! ## Quick Start
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! async-retry-project = { path = "path/to/async-retry-project" }
//! # Enable your runtime (e.g., Tokio)
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! **Note:** You *must* enable a timer feature for this crate:
//! `features = ["tokio-timer"]` or `features = ["async-std-timer"]`.
//!
//! ### Example: Simple Retry
//!
//! ```rust,no_run
//! use async_retry_project::{Retry, backoff::ExponentialBackoff};
//! use std::time::Duration;
//!
//! // A mock function that might fail
//! async fn fetch_data() -> Result<String, String> {
//!     // ... logic that might fail
//!     Err("Failed to connect".to_string())
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let strategy = ExponentialBackoff::new(Duration::from_millis(100))
//!         .with_max_retries(5);
//!
//!     let operation = || async {
//!         fetch_data().await
//!     };
//!
//!     let result = Retry::new(strategy, operation).await;
//!
//!     match result {
//!         Ok(data) => println!("Succeeded: {}", data),
//!         Err(e) => println!("Failed after retries: {}", e),
//!     }
//! }
//! ```
//!
//! ### Example: Conditional Retry
//!
//! ```rust,no_run
//! use async_retry_project::{Retry, backoff::ExponentialBackoff};
//! use std::time::Duration;
//!
//! // Define a custom error
//! #[derive(Debug, Clone)]
//! enum MyError {
//!     TransientNetworkError,
//!     PermanentAuthError,
//! }
//!
//! async fn fetch_sensitive_data() -> Result<String, MyError> {
//!     // ...
//!     Err(MyError::TransientNetworkError)
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let strategy = ExponentialBackoff::new(Duration::from_millis(200))
//!         .with_max_retries(3);
//!
//!     // Only retry on transient errors
//!     let condition = |e: &MyError| {
//!         matches!(e, MyError::TransientNetworkError)
//!     };
//!
//!     let operation = || async { fetch_sensitive_data().await };
//!
//!     let result = Retry::new(strategy, operation)
//!         .with_condition(condition)
//!         .await;
//!
//!     if let Err(MyError::PermanentAuthError) = result {
//!         println!("Failed immediately due to auth error.");
//!     }
//! }
//! ```

// Public modules
pub mod backoff;
mod sleep;

// Public re-exports for easier use
pub use backoff::{Backoff, ExponentialBackoff, FibonacciBackoff, FixedDelay};

#[cfg(feature = "jitter")]
pub use backoff::Jitter;

use std::future::IntoFuture;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

/// A predicate function that always returns true, retryable for all errors.
fn default_condition(_: &dyn Error) -> bool {
    true
}

/// The main builder struct for a retryable operation.
///
/// This struct is created by [`Retry::new()`] and configured using its
/// "builder" style methods like [`with_condition()`] and [`with_max_duration()`].
///
/// It implements `IntoFuture`, so you can simply `.await` it.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Retry<S, O, C>
where
    S: Backoff,
{
    strategy: S,
    operation: O,
    condition: C,
    max_duration: Option<Duration>,
}

// Implementation block for creating a new Retry with the default condition.
impl<S, O> Retry<S, O, fn(&dyn Error) -> bool>
where
    S: Backoff,
{
    /// Creates a new `Retry` instance.
    ///
    /// - `strategy`: A [`Backoff`] strategy iterator (e.g., [`ExponentialBackoff`]).
    /// - `operation`: A closure that returns a `Future` (e.g., `|| async { ... }`).
    ///
    /// By default, it retries on *all* errors. Use [`with_condition()`] to change this.
    pub fn new(strategy: S, operation: O) -> Self {
        Self {
            strategy,
            operation,
            condition: default_condition,
            max_duration: None,
        }
    }
}

// Implementation block for builder methods, available on any Retry instance.
impl<S, O, C> Retry<S, O, C>
where
    S: Backoff,
{
    /// Sets a new condition predicate for retrying.
    ///
    /// The closure `condition` receives a reference to the error `&E` and
    /// must return `true` if a retry should be attempted, or `false`
    /// if the loop should give up and return the error.
    pub fn with_condition<NewC, E>(self, condition: NewC) -> Retry<S, O, NewC>
    where
        NewC: FnMut(&E) -> bool,
        E: Error,
    {
        Retry {
            strategy: self.strategy,
            operation: self.operation,
            condition,
            max_duration: self.max_duration,
        }
    }

    /// Sets a maximum total duration for the entire retry operation.
    ///
    /// If the total time (including retries and delays) exceeds this
    /// duration, the loop will stop and return the last error.
    ///
    /// Fulfills FR6.
    pub fn with_max_duration(mut self, max_duration: Duration) -> Self {
        self.max_duration = Some(max_duration);
        self
    }
}

/// The core retry logic, implemented via `IntoFuture`.
///
/// This allows `Retry` to be `.await`ed directly.
impl<S, O, C, F, T, E> IntoFuture for Retry<S, O, C>
where
    S: Backoff + Send + 'static,
    O: FnMut() -> F + Send + 'static,
    C: FnMut(&E) -> bool + Send + 'static,
    F: Future<Output = Result<T, E>> + Send,
    E: Error + Send,
    T: Send,
{
    type Output = Result<T, E>;

    // We box the future to avoid complex type signatures in the return.
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;

    /// Contains the core retry loop logic.
    fn into_future(mut self) -> <Retry<S, O, C> as IntoFuture>::IntoFuture {
        Box::pin(async move {
            let start_time = Instant::now();
            let mut attempt = 0;

            loop {
                attempt += 1;

                // Execute the async operation.
                let result = (self.operation)().await;

                match result {
                    // Success, return the value.
                    Ok(value) => {
                        #[cfg(feature = "logging")]
                        log::trace!("Operation succeeded on attempt {}", attempt);
                        return Ok(value);
                    }
                    // Failure, check if we should retry.
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        log::warn!(
                            "Operation failed on attempt {} with error: {}",
                            attempt,
                            e
                        );

                        // Check max total duration limit
                        if let Some(max_duration) = self.max_duration {
                            if start_time.elapsed() >= max_duration {
                                #[cfg(feature = "logging")]
                                log::error!(
                                    "Retry failed: max duration ({:?}) exceeded.",
                                    max_duration
                                );
                                return Err(e); // Exhausted time
                            }
                        }

                        // Check the retry condition
                        if !(self.condition)(&e) {
                            #[cfg(feature = "logging")]
                            log::error!("Retry failed: error is not retryable.");
                            return Err(e); // Not a retryable error
                        }

                        // Get next backoff duration
                        // This also implicitly handles (Max Retries) if the
                        // strategy itself is limited (e.g., via `.take(n)` or
                        // `with_max_retries()`).
                        if let Some(delay) = self.strategy.next() {
                            // Check if the *sleep itself* would exceed max duration
                            if let Some(max_duration) = self.max_duration {
                                if start_time.elapsed() + delay > max_duration {
                                    #[cfg(feature = "logging")]
                                    log::error!(
                                        "Retry failed: next delay ({:?}) would exceed max duration.",
                                        delay
                                    );
                                    return Err(e); // Sleep would exceed total duration
                                }
                            }

                            // Perform the runtime-agnostic sleep
                            #[cfg(feature = "logging")]
                            log::trace!("Retrying after delay of {:?}", delay);
                            sleep::sleep(delay).await;
                        } else {
                            // Backoff strategy is exhausted
                            #[cfg(feature = "logging")]
                            log::error!(
                                "Retry failed: backoff strategy exhausted after {} attempts.",
                                attempt
                            );
                            return Err(e);
                        }
                    }
                }
            }
        })
    }
}
#[cfg(all(test, feature = "tokio-timer"))]
mod tests {
    use super::*;
    use crate::backoff::FixedDelay;
    use std::time::Duration;
    
    #[derive(Debug, Clone, PartialEq)]
    struct TestError(String);
    
    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    
    impl std::error::Error for TestError {}

    #[tokio::test]
    async fn test_simple_retry() {
        let strategy = FixedDelay::new(Duration::from_millis(10)).take(3);
        
        let operation = || async {
            Ok::<u32, TestError>(42)
        };
        
        let result = Retry::new(strategy, operation).await;
        assert_eq!(result, Ok(42));
    }
}
