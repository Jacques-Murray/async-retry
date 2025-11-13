// Author: Jacques Murray
//
// Common code for examples, such as custom errors.
// Used by conditional_retry.rs

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
