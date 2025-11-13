// Author: Jacques Murray

//! Defines the `Backoff` trait and built-in backoff strategies.
//!
//! A `Backoff` is simply an `Iterator` that yields `Duration`s.
//! When the iterator returns `None`, the retry loop stops.

use std::time::Duration;

/// Trait for backoff strategies.
///
/// This is implemented as an `Iterator` over `Duration`.
/// When the iterator returns `None`, the retry loop will stop.
///
/// You can use standard `Iterator` adapters like `.take(n)` to
/// limit the number of retries.
pub trait Backoff: Iterator<Item = Duration> {}

// Implement the trait for all types that fit the criteria.
impl<T> Backoff for T where T: Iterator<Item = Duration> {}

// --- Fixed Delay Strategy ---

/// A backoff strategy that waits for a fixed duration.
/// This iterator is infinite unless limited (e.g., with `.take()`).
#[derive(Debug, Clone, Copy)]
pub struct FixedDelay {
    duration: Duration,
}

impl FixedDelay {
    /// Creates a new `FixedDelay` strategy.
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

impl Iterator for FixedDelay {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.duration)
    }
}

// --- Exponential Backoff Strategy ---

/// A backoff strategy that doubles the wait duration.
///
/// Example: 100ms, 200ms, 400ms, 800ms...
#[derive(Debug, Clone, Copy)]
pub struct ExponentialBackoff {
    current: Duration,
    base: Duration,
    max_delay: Option<Duration>,
    max_retries: Option<usize>,
    attempt: usize,
}

impl ExponentialBackoff {
    /// Creates a new `ExponentialBackoff` strategy.
    ///
    /// - `base_delay`: The initial delay (e.g., 100ms).
    pub fn new(base_delay: Duration) -> Self {
        Self {
            current: base_delay,
            base: base_delay,
            max_delay: None,
            max_retries: None,
            attempt: 0,
        }
    }

    /// Sets an optional maximum delay.
    /// The backoff will not increase beyond this duration.
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = Some(max_delay);
        self
    }

    /// Sets an optional maximum number of retries.
    /// The iterator will return `None` after this many attempts.
    /// Fulfills requirement from API Example 1.
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = Some(max_retries);
        self
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        // Check max retries limit
        if let Some(max_retries) = self.max_retries {
            if self.attempt >= max_retries {
                return None;
            }
        }
        self.attempt += 1;

        // Get the current delay
        let mut delay = self.current;

        // Apply max delay cap
        if let Some(max_delay) = self.max_delay {
            delay = delay.min(max_delay);
        }

        // Calculate next duration
        // We use saturating_mul to prevent panic on overflow.
        self.current = self.current.saturating_mul(2);

        Some(delay)
    }
}

/// --- Fibonacci Backoff Strategy ---

/// A backoff strategy based on the Fibonacci sequence.
///
/// Example: 1s, 1s, 2s, 3s, 5s, 8s...
#[derive(Debug, Clone, Copy)]
pub struct FibonacciBackoff {
    current: Duration,
    next: Duration,
    max_delay: Option<Duration>,
    max_retries: Option<usize>,
    attempt: usize,
}

impl FibonacciBackoff {
    /// Creates a new `FibonacciBackoff`.
    ///
    /// - `base_delay`: The duration for the first two retries (e.g., 1s).
    pub fn new(base_delay: Duration) -> Self {
        Self {
            current: base_delay,
            next: base_delay,
            max_delay: None,
            max_retries: None,
            attempt: 0,
        }
    }

    /// Sets an optional maximum delay.
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = Some(max_delay);
        self
    }

    /// Sets an optional maximum number of retries.
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = Some(max_retries);
        self
    }
}

impl Iterator for FibonacciBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        // Check max retries limit
        if let Some(max_retries) = self.max_retries {
            if self.attempt >= max_retries {
                return None;
            }
        }
        self.attempt += 1;

        // Get the current delay
        let mut delay = self.current;

        // Apply max delay cap
        if let Some(max_delay) = self.max_delay {
            delay = delay.min(max_delay);
        }

        // Calculate next duration
        let new_next = self.current.saturating_add(self.next);
        self.current = self.next;
        self.next = new_next;

        Some(delay)
    }
}

// --- Jitter (Future Work) ---

/// A wrapper that adds random jitter to any `Backoff` strategy.
///
/// This is crucial for production systems to prevent the "thundering herd"
/// problem. It requires the `jitter` feature flag.
#[cfg(feature = "jitter")]
#[derive(Debug, Clone)]
pub struct Jitter<B: Backoff> {
    inner: B,
}

#[cfg(feature = "jitter")]
impl<B: Backoff> Jitter<B> {
    /// Wraps a `Backoff` strategy to add full jitter.
    ///
    /// The jitter applied is a random duration between 0 and the
    /// duration provided by the inner strategy.
    pub fn new(inner: B) -> Self {
        Self { inner }
    }
}

#[cfg(feature = "jitter")]
impl<B: Backoff> Iterator for Jitter<B> {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|duration| {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            // Apply full jitter: 0..=duration
            let jitter_millis = rng.gen_range(0..=duration.as_millis());
            Duration::from_millis(jitter_millis as u64)
        })
    }
}

// --- Unit Tests (as required by persona) ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_delay() {
        let mut strategy = FixedDelay::new(Duration::from_secs(1)).take(3);
        assert_eq!(strategy.next(), Some(Duration::from_secs(1)));
        assert_eq!(strategy.next(), Some(Duration::from_secs(1)));
        assert_eq!(strategy.next(), Some(Duration::from_secs(1)));
        assert_eq!(strategy.next(), None);
    }

    #[test]
    fn test_exponential_backoff() {
        let mut strategy = ExponentialBackoff::new(Duration::from_millis(100)).take(4);
        assert_eq!(strategy.next(), Some(Duration::from_millis(100)));
        assert_eq!(strategy.next(), Some(Duration::from_millis(200)));
        assert_eq!(strategy.next(), Some(Duration::from_millis(400)));
        assert_eq!(strategy.next(), Some(Duration::from_millis(800)));
        assert_eq!(strategy.next(), None);
    }

    #[test]
    fn test_exponential_backoff_with_max_delay() {
        let mut strategy = ExponentialBackoff::new(Duration::from_millis(100))
            .with_max_delay(Duration::from_millis(300))
            .take(4);
        assert_eq!(strategy.next(), Some(Duration::from_millis(100)));
        assert_eq!(strategy.next(), Some(Duration::from_millis(200)));
        assert_eq!(strategy.next(), Some(Duration::from_millis(300))); // Capped
        assert_eq!(strategy.next(), Some(Duration::from_millis(300))); // Capped
        assert_eq!(strategy.next(), None);
    }

    #[test]
    fn test_exponential_backoff_with_max_retries() {
        let mut strategy = ExponentialBackoff::new(Duration::from_millis(100)).with_max_retries(2);
        assert_eq!(strategy.next(), Some(Duration::from_millis(100)));
        assert_eq!(strategy.next(), Some(Duration::from_millis(200)));
        assert_eq!(strategy.next(), None); // Limit reached
    }

    #[test]
    fn test_fibonacci_backoff() {
        let mut strategy = FibonacciBackoff::new(Duration::from_secs(1)).take(6);
        assert_eq!(strategy.next(), Some(Duration::from_secs(1)));
        assert_eq!(strategy.next(), Some(Duration::from_secs(1)));
        assert_eq!(strategy.next(), Some(Duration::from_secs(2)));
        assert_eq!(strategy.next(), Some(Duration::from_secs(3)));
        assert_eq!(strategy.next(), Some(Duration::from_secs(5)));
        assert_eq!(strategy.next(), Some(Duration::from_secs(8)));
        assert_eq!(strategy.next(), None);
    }

    #[test]
    fn test_fibonacci_backoff_with_max_retries() {
        let mut strategy = FibonacciBackoff::new(Duration::from_secs(1)).with_max_retries(3);
        assert_eq!(strategy.next(), Some(Duration::from_secs(1)));
        assert_eq!(strategy.next(), Some(Duration::from_secs(1)));
        assert_eq!(strategy.next(), Some(Duration::from_secs(2)));
        assert_eq!(strategy.next(), None); // Limit reached
    }

    #[cfg(feature = "jitter")]
    #[test]
    fn test_jitter_wrapper() {
        let fixed = FixedDelay::new(Duration::from_secs(1));
        let mut jitter = Jitter::new(fixed).take(10);
        for _ in 0..10 {
            let duration = jitter.next().unwrap();
            assert!(duration <= Duration::from_secs(1));
        }
        assert_eq!(jitter.next(), None);
    }
}
