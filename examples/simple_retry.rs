// Author: Jacques Murray

use async_retry::{backoff::ExponentialBackoff, Retry};
use std::time::{Duration, Instant};
use thiserror::Error;

// Define a proper error type
#[derive(Debug, Error)]
#[error("Operation failed: {message}")]
struct OperationError {
    message: String,
}

// A mock function that will fail 3 times before succeeding.
async fn flaky_operation() -> Result<String, OperationError> {
    // Use a static to track attempts across calls
    static ATTEMPTS: tokio::sync::Mutex<u32> = tokio::sync::Mutex::const_new(0);

    let mut attempts = ATTEMPTS.lock().await;
    *attempts += 1;

    println!("Attempt {}: Trying operation...", *attempts);

    if *attempts <= 3 {
        println!("Attempt {}: Failed.", *attempts);
        Err(OperationError {
            message: format!("Failed on attempt {}", *attempts),
        })
    } else {
        println!("Attempt {}: Succeeded.", *attempts);
        Ok("Got the data!".to_string())
    }
}

#[tokio::main]
async fn main() {
    println!("--- Running Simple Retry Example ---");

    // Simple retry with exponential backoff
    let strategy = ExponentialBackoff::new(Duration::from_millis(100))
        .with_max_retries(5); // Stop after 5 total attempts

    let start = Instant::now();

    // The operation is a closure that returns the async block (Future)
    let operation = || async { flaky_operation().await };

    let result = Retry::new(strategy, operation).await;

    println!("\n--- Result ---");
    match result {
        Ok(data) => println!("Success: {}", data),
        Err(e) => println!("Failed: {}", e),
    }
    println!("Total time: {:?}", start.elapsed());
}