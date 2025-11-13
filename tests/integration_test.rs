// Author: Jacques Murray

use async_retry::{backoff::FixedDelay, Retry};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// A simple error for testing
#[derive(Debug, PartialEq, Eq, Clone)]
struct TestError(String);

// Implement Error for our test error
impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for TestError {}

// A stateful operation for testing
#[derive(Clone)]
struct Op {
    // Use Arc<AtomicU32> for thread-safe interior mutability
    attempts: Arc<AtomicU32>,
    succeed_on: u32,
    error_to_return: TestError,
}

impl Op {
    fn new(succeed_on: u32, error: &str) -> Self {
        Self {
            attempts: Arc::new(AtomicU32::new(0)),
            succeed_on,
            error_to_return: TestError(error.to_string()),
        }
    }

    // The operation itself
    async fn run(&self) -> Result<u32, TestError> {
        let current = self.attempts.fetch_add(1, Ordering::SeqCst) + 1;

        if current == self.succeed_on {
            Ok(current)
        } else {
            // Clone the error to return it
            Err(TestError(self.error_to_return.0.clone()))
        }
    }

    fn attempts(&self) -> u32 {
        self.attempts.load(Ordering::SeqCst)
    }
}

#[tokio::test]
async fn test_success_on_first_try() {
    let op = Op::new(1, "fail"); // Succeeds on attempt 1
    let strategy = FixedDelay::new(Duration::from_millis(10)).take(5);

    let op_clone = op.clone();
    let result = Retry::new(strategy, move || {
        let op = op_clone.clone();
        async move { op.run().await }
    }).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
    assert_eq!(op.attempts(), 1);
}

#[tokio::test]
async fn test_success_on_third_try() {
    let op = Op::new(3, "fail"); // Succeeds on attempt 3
    let strategy = FixedDelay::new(Duration::from_millis(10)).take(5);

    let op_clone = op.clone();
    let result = Retry::new(strategy, move || {
        let op = op_clone.clone();
        async move { op.run().await }
    }).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 3);
    assert_eq!(op.attempts(), 3);
}

#[tokio::test]
async fn test_failure_on_max_retries() {
    // Max Retries
    let op = Op::new(10, "fail"); // Succeeds on attempt 10
    let strategy = FixedDelay::new(Duration::from_millis(10)).take(3); // 3 retries = 4 total attempts (1 initial + 3 retries)

    let start = Instant::now();
    let op_clone = op.clone();
    let result = Retry::new(strategy, move || {
        let op = op_clone.clone();
        async move { op.run().await }
    }).await;

    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), TestError("fail".to_string()));
    assert_eq!(op.attempts(), 4); // 1 initial attempt + 3 retries = 4 total attempts
    // Check that it slept 3 times (10ms + 10ms + 10ms)
    assert!(elapsed >= Duration::from_millis(30));
}

#[tokio::test]
async fn test_failure_on_max_duration() {
    // Max Duration
    let op = Op::new(10, "fail"); // Succeeds on 10
    // Strategy allows 10 retries, but each sleeps 50ms
    let strategy = FixedDelay::new(Duration::from_millis(50)).take(10);

    let op_clone = op.clone();
    let result = Retry::new(strategy, move || {
        let op = op_clone.clone();
        async move { op.run().await }
    })
        .with_max_duration(Duration::from_millis(75)) // Max duration is 75ms
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), TestError("fail".to_string()));
    // Should run once (fail), sleep (50ms), run twice (fail).
    // The *next* sleep (50ms) would exceed 75ms, so it stops.
    assert_eq!(op.attempts(), 2);
}

#[tokio::test]
async fn test_failure_on_condition() {
    // Retry Conditions
    let op = Op::new(10, "PERMANENT"); // Fails with "PERMANENT"
    let strategy = FixedDelay::new(Duration::from_millis(10)).take(5);

    let condition = |e: &TestError| e.0 != "PERMANENT";

    let op_clone = op.clone();
    let result = Retry::new(strategy, move || {
        let op = op_clone.clone();
        async move { op.run().await }
    })
        .with_condition(condition)
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), TestError("PERMANENT".to_string()));
    // Should fail on the very first attempt
    assert_eq!(op.attempts(), 1);
}