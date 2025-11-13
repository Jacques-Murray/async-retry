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
//! async-retry = { path = "path/to/async-retry" }
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
//! use async_retry::{Retry, backoff::ExponentialBackoff};
//! use std::time::Duration;
//! use thiserror::Error;
//!
//! #[derive(Debug, Error)]
//! #[error("Failed to connect: {0}")]
//! struct ConnectionError(String);
//!
//! // Define a simple error type
//! #[derive(Debug, Clone)]
//! struct MyError(String);
//!
//! impl std::fmt::Display for MyError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         write!(f, "{}", self.0)
//!     }
//! }
//!
//! impl std::error::Error for MyError {}
//!
//! // A mock function that might fail
//! async fn fetch_data() -> Result<String, ConnectionError> {
//!     // ... logic that might fail
//!     Err(ConnectionError("Network error".to_string()))
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let strategy = ExponentialBackoff::new(Duration::from_millis(100))
//!         .with_max_retries(5);
//!
//!     let operation = move || async move {
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
//! use async_retry::{Retry, backoff::ExponentialBackoff};
//! use std::time::Duration;
//!
//! // Define a custom error
//! #[derive(Debug, Clone)]
//! enum MyError {
//!     TransientNetworkError,
//!     PermanentAuthError,
//! }
//!
//! impl std::fmt::Display for MyError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         match self {
//!             MyError::TransientNetworkError => write!(f, "Network error"),
//!             MyError::PermanentAuthError => write!(f, "Auth error"),
//!         }
//!     }
//! }
//!
//! impl std::error::Error for MyError {}
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
//!     let operation = move || async move { fetch_sensitive_data().await };
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

use std::error::Error;
use std::future::Future;
use std::future::IntoFuture;
use std::future::IntoFuture;
use std::pin::Pin;
use std::time::{Duration, Instant};

/// A predicate function that always returns true, retryable for all errors.
fn default_condition(_: &dyn Error) -> bool {
    true
}

/// The main builder struct for retryable operations.
///
/// `Retry` provides a fluent builder API for configuring retry behavior. It is generic
/// over three type parameters:
///
/// - `S`: The backoff strategy (implements [`Backoff`])
/// - `O`: The operation closure that returns a future
/// - `C`: The condition function that determines if an error should be retried
///
/// # Type Parameters
///
/// The type parameters are automatically inferred from the arguments passed to
/// [`Retry::new()`] and builder methods, so you typically don't need to specify them.
///
/// # Builder Methods
///
/// - [`new()`](Retry::new) - Creates a new retry instance with default "retry all" behavior
/// - [`with_condition()`](Retry::with_condition) - Sets a custom retry condition
/// - [`with_max_duration()`](Retry::with_max_duration) - Sets a maximum total duration
///
/// # Execution
///
/// `Retry` implements [`IntoFuture`], which means you can `.await` it directly:
///
/// ```rust,no_run
/// # use async_retry::{Retry, backoff::FixedDelay};
/// # use std::time::Duration;
/// # #[derive(Debug, Clone)]
/// # struct MyError;
/// # impl std::fmt::Display for MyError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
/// # }
/// # impl std::error::Error for MyError {}
/// # async fn operation() -> Result<(), MyError> { Ok(()) }
/// # async fn example() {
/// let strategy = FixedDelay::new(Duration::from_secs(1)).take(3);
/// let result = Retry::new(strategy, move || async move { operation().await }).await;
/// # }
/// ```
///
/// # Closure Requirements
///
/// The operation closure must:
/// - Return a `Future` that produces a `Result<T, E>`
/// - Be `Send + 'static` for thread safety
/// - Be `FnMut` so it can be called multiple times
///
/// To satisfy these requirements, use `move || async move { ... }` pattern:
///
/// ```rust,no_run
/// # use async_retry::{Retry, backoff::FixedDelay};
/// # use std::time::Duration;
/// # #[derive(Debug, Clone)]
/// # struct MyError;
/// # impl std::fmt::Display for MyError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
/// # }
/// # impl std::error::Error for MyError {}
/// # async fn fetch() -> Result<String, MyError> { Ok(String::new()) }
/// # async fn example() {
/// let operation = move || async move { fetch().await };
/// let result = Retry::new(FixedDelay::new(Duration::from_secs(1)), operation).await;
/// # }
/// ```
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
impl<S, O> Retry<S, O, AlwaysRetry>
where
    S: Backoff,
{
    /// Creates a new `Retry` instance that retries on *all* errors.
    ///
    /// # Arguments
    ///
    /// * `strategy` - A [`Backoff`] strategy that controls retry timing
    /// * `operation` - A closure returning a `Future<Output = Result<T, E>>`
    ///
    /// # Returns
    ///
    /// A `Retry` builder that can be configured further or awaited directly.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use async_retry::{Retry, backoff::ExponentialBackoff};
    /// use std::time::Duration;
    ///
    /// # #[derive(Debug, Clone)]
    /// # struct MyError;
    /// # impl std::fmt::Display for MyError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
    /// # }
    /// # impl std::error::Error for MyError {}
    /// # async fn fetch_data() -> Result<String, MyError> { Ok(String::new()) }
    /// # async fn example() {
    /// let strategy = ExponentialBackoff::new(Duration::from_millis(100))
    ///     .with_max_retries(5);
    ///
    /// let result = Retry::new(strategy, move || async move {
    ///     fetch_data().await
    /// }).await;
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// * [`with_condition()`](Retry::with_condition) - Add custom retry logic
    /// * [`with_max_duration()`](Retry::with_max_duration) - Set time limit
    pub fn new(strategy: S, operation: O) -> Self {
        Self {
            strategy,
            operation,
            condition: AlwaysRetry,
            max_duration: None,
        }
    }
}

// Implementation block for builder methods, available on any Retry instance.
impl<S, O, C> Retry<S, O, C>
where
    S: Backoff,
{
    /// Sets a custom condition for determining which errors should be retried.
    ///
    /// By default, [`Retry::new()`] retries all errors. Use this method to specify
    /// custom logic for which errors are retryable.
    ///
    /// # Arguments
    ///
    /// * `condition` - A closure `Fn(&E) -> bool` that returns `true` for retryable errors
    ///
    /// # Returns
    ///
    /// A new `Retry` instance with the specified condition.
    ///
    /// # Examples
    ///
    /// Only retry on network errors:
    ///
    /// ```rust,no_run
    /// use async_retry::{Retry, backoff::FixedDelay};
    /// use std::time::Duration;
    ///
    /// # #[derive(Debug, Clone)]
    /// # enum ApiError {
    /// #     Network,
    /// #     Auth,
    /// # }
    /// # impl std::fmt::Display for ApiError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
    /// # }
    /// # impl std::error::Error for ApiError {}
    /// # async fn call_api() -> Result<(), ApiError> { Ok(()) }
    /// # async fn example() {
    /// let condition = |e: &ApiError| matches!(e, ApiError::Network);
    ///
    /// let result = Retry::new(
    ///     FixedDelay::new(Duration::from_secs(1)).take(3),
    ///     move || async move { call_api().await }
    /// )
    /// .with_condition(condition)
    /// .await;
    /// # }
    /// ```
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
    /// # Arguments
    ///
    /// * `max_duration` - The maximum total time to spend retrying
    ///
    /// # Behavior
    ///
    /// The retry loop checks the elapsed time:
    /// 1. Before waiting for a backoff delay
    /// 2. If the delay would cause the total time to exceed `max_duration`, the loop stops
    ///
    /// # Examples
    ///
    /// Limit retries to 10 seconds total:
    ///
    /// ```rust,no_run
    /// use async_retry::{Retry, backoff::FixedDelay};
    /// use std::time::Duration;
    ///
    /// # #[derive(Debug, Clone)]
    /// # struct MyError;
    /// # impl std::fmt::Display for MyError {
    /// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
    /// # }
    /// # impl std::error::Error for MyError {}
    /// # async fn operation() -> Result<(), MyError> { Ok(()) }
    /// # async fn example() {
    /// // Even though the strategy allows many retries, this will stop after 10 seconds
    /// let result = Retry::new(
    ///     FixedDelay::new(Duration::from_secs(1)),  // Infinite retries
    ///     move || async move { operation().await }
    /// )
    /// .with_max_duration(Duration::from_secs(10))  // But stop after 10 seconds
    /// .await;
    /// # }
    /// ```
    pub fn with_max_duration(mut self, max_duration: Duration) -> Self {
        self.max_duration = Some(max_duration);
        self
    }
}

/// The core retry logic, implemented via `IntoFuture` for the default (always retry) condition.
impl<S, O, F, T, E> IntoFuture for Retry<S, O, AlwaysRetry>
where
    S: Backoff + Send + 'static,
    O: FnMut() -> F + Send + 'static,
    F: Future<Output = Result<T, E>> + Send,
    E: Error + Send,
    T: Send,
{
    type Output = Result<T, E>;

    // We box the future to avoid complex type signatures in the return.
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;

    /// Contains the core retry loop logic.
    fn into_future(mut self) -> <Retry<S, O, AlwaysRetry> as IntoFuture>::IntoFuture {
        Box::pin(async move {
            let start_time = Instant::now();
            let mut _attempt = 0;

            loop {
                _attempt += 1;

                // Execute the async operation.
                let result = (self.operation)().await;

                match result {
                    // Success, return the value.
                    Ok(value) => {
                        #[cfg(feature = "logging")]
                        log::trace!("Operation succeeded on attempt {}", _attempt);
                        return Ok(value);
                    }
                    // Failure, check if we should retry.
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        log::warn!(
                            "Operation failed on attempt {} with error: {}",
                            _attempt,
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

                        // Always retry with AlwaysRetry condition

                        // Get next backoff duration
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
                                _attempt
                            );
                            return Err(e);
                        }
                    }
                }
            }
        })
    }
}

/// The core retry logic, implemented via `IntoFuture` for custom conditions.
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
            #[allow(unused_variables)]
            let mut attempt = 0;

            loop {
                _attempt += 1;

                // Execute the async operation.
                let result = (self.operation)().await;

                match result {
                    // Success, return the value.
                    Ok(value) => {
                        #[cfg(feature = "logging")]
                        log::trace!("Operation succeeded on attempt {}", _attempt);
                        return Ok(value);
                    }
                    // Failure, check if we should retry.
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        log::warn!("Operation failed on attempt {} with error: {}", _attempt, e);

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
                                _attempt
                            );
                            return Err(e);
                        }
                    }
                }
            }
        })
    }
}
