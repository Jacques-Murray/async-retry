// Author: Jacques Murray

use async_retry::{backoff::ExponentialBackoff, Retry};
use std::time::Duration;
use thiserror::Error;

/// A custom error type for the `reqwest` examples.
///
/// This demonstrates how to implement a condition function
/// for a specific error type.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Network connection error: {0}")]
    Connection(#[from] reqwest::Error),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Client error (not retryable): {0}")]
    ClientError(String),
}

/// The condition function for the example.
///
/// This implements the logic from the PRD's Example 2:
/// "Only retry on transient network errors... or server errors."
pub fn should_retry_api_error(e: &ApiError) -> bool {
    match e {
        // Retry on network errors
        ApiError::Connection(_) => true,
        // Retry on 5xx server errors
        ApiError::ServerError(_) => true,
        // DO NOT retry on 4xx client errors
        ApiError::ClientError(_) => false,
    }
}

/// A mock API fetcher.
/// We use `httpstat.us` to force specific HTTP error codes.
async fn fetch_important_data(status_code: u16) -> Result<String, ApiError> {
    let url = format!("https://httpstat.us/{}", status_code);
    println!("Fetching {}...", url);

    let res = reqwest::get(&url).await.map_err(ApiError::Connection)?;

    let status = res.status();
    let text = res.text().await.map_err(ApiError::Connection)?;

    match status {
        s if s.is_success() => Ok(text),
        s if s.is_client_error() => Err(ApiError::ClientError(format!("{}: {}", s, text))),
        s if s.is_server_error() => Err(ApiError::ServerError(format!("{}: {}", s, text))),
        _ => Err(ApiError::ServerError("Unknown error".to_string())),
    }
}

async fn run_example(code: u16, desc: &str) {
    println!("\n--- Running Conditional Retry: {} ({}) ---", desc, code);

    // Retry with a condition
    let strategy = ExponentialBackoff::new(Duration::from_millis(200))
        .with_max_retries(3); // 3 retries = 3 total attempts

    // The operation closure captures the status code
    let operation = move || async move { fetch_important_data(code).await };

    let result = Retry::new(strategy, operation)
        .with_condition(should_retry_api_error) // Use our custom condition
        .await;

    match result {
        Ok(data) => println!("Success: {}", data),
        Err(e) => println!("Failed: {}", e),
    }
}

#[tokio::main]
async fn main() {
    // 1. Test server error (should retry and fail)
    // 503 Service Unavailable is retryable
    run_example(503, "Server Error (503)").await;

    // 2. Test client error (should fail immediately)
    // 404 Not Found is NOT retryable
    run_example(404, "Client Error (404)").await;
}