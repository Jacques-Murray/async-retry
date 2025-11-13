# async-retry

[![Crates.io](https://img.shields.io/crates/v/async-retry.svg)](https://crates.io/crates/async-retry)
[![Documentation](https://docs.rs/async-retry/badge.svg)](https://docs.rs/async-retry)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A Rust library for retrying asynchronous operations with customizable backoff strategies.

## Features

- **Simple, ergonomic API** for retrying async operations
- **Flexible backoff strategies**: Fixed delay, Exponential, Fibonacci
- **Conditional retries**: Retry only on specific error types
- **Runtime-agnostic**: Supports Tokio and async-std via feature flags
- **Configurable limits**: Maximum retries, maximum duration, and custom conditions
- **Optional jitter**: Prevent thundering herd problems (with `jitter` feature)
- **Optional logging**: Integrated logging support (with `logging` feature)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
async-retry = { version = "0.1", features = ["tokio-timer"] }
tokio = { version = "1", features = ["full"] }
```

### Feature Flags

You **must** enable one timer feature:
- `tokio-timer`: Use Tokio's timer (requires Tokio runtime)
- `async-std-timer`: Use async-std's timer (requires async-std runtime)

Optional features:
- `jitter`: Enable jitter support for backoff strategies
- `logging`: Enable logging via the `log` crate

## Quick Start

### Simple Retry with Exponential Backoff

```rust
use async_retry::{Retry, backoff::ExponentialBackoff};
use std::time::Duration;

#[derive(Debug)]
struct MyError(String);

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MyError {}

async fn fetch_data() -> Result<String, MyError> {
    // Your async operation that might fail
    Err(MyError("Failed to connect".to_string()))
}

#[tokio::main]
async fn main() {
    let strategy = ExponentialBackoff::new(Duration::from_millis(100))
        .with_max_retries(5);

    let operation = move || async move { fetch_data().await };

    let result = Retry::new(strategy, operation).await;

    match result {
        Ok(data) => println!("Success: {}", data),
        Err(e) => println!("Failed after retries: {}", e),
    }
}
```

### Conditional Retry

Only retry on specific error types:

```rust
use async_retry::{Retry, backoff::ExponentialBackoff};
use std::time::Duration;

#[derive(Debug, Clone)]
enum ApiError {
    TransientNetworkError,
    PermanentAuthError,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::TransientNetworkError => write!(f, "Transient network error"),
            ApiError::PermanentAuthError => write!(f, "Permanent auth error"),
        }
    }
}

impl std::error::Error for ApiError {}

async fn fetch_sensitive_data() -> Result<String, ApiError> {
    Err(ApiError::TransientNetworkError)
}

#[tokio::main]
async fn main() {
    let strategy = ExponentialBackoff::new(Duration::from_millis(200))
        .with_max_retries(3);

    // Only retry on transient errors
    let condition = |e: &ApiError| {
        matches!(e, ApiError::TransientNetworkError)
    };

    let operation = move || async move { fetch_sensitive_data().await };

    let result = Retry::new(strategy, operation)
        .with_condition(condition)
        .await;

    match result {
        Ok(data) => println!("Success: {}", data),
        Err(ApiError::PermanentAuthError) => {
            println!("Failed immediately due to auth error");
        }
        Err(e) => println!("Failed after retries: {}", e),
    }
}
```

### Maximum Duration

Limit the total time spent retrying:

```rust
use async_retry::{Retry, backoff::FixedDelay};
use std::time::Duration;

# #[derive(Debug, Clone)]
# struct MyError(String);
# impl std::fmt::Display for MyError {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#         write!(f, "{}", self.0)
#     }
# }
# impl std::error::Error for MyError {}
# async fn fetch_data() -> Result<String, MyError> {
#     Err(MyError("Error".to_string()))
# }

#[tokio::main]
async fn main() {
    let strategy = FixedDelay::new(Duration::from_secs(1));

    let operation = move || async move { fetch_data().await };

    let result = Retry::new(strategy, operation)
        .with_max_duration(Duration::from_secs(10))
        .await;

    // Will stop after 10 seconds, even if more retries are available
}
```

## Backoff Strategies

### Fixed Delay

Waits a fixed duration between retries:

```rust
use async_retry::backoff::FixedDelay;
use std::time::Duration;

let strategy = FixedDelay::new(Duration::from_secs(1))
    .take(5); // Limit to 5 retries
```

### Exponential Backoff

Doubles the delay after each retry:

```rust
use async_retry::backoff::ExponentialBackoff;
use std::time::Duration;

let strategy = ExponentialBackoff::new(Duration::from_millis(100))
    .with_max_delay(Duration::from_secs(30))  // Cap at 30 seconds
    .with_max_retries(10);                    // Limit retries
```

### Fibonacci Backoff

Uses Fibonacci sequence for delays:

```rust
use async_retry::backoff::FibonacciBackoff;
use std::time::Duration;

let strategy = FibonacciBackoff::new(Duration::from_secs(1))
    .with_max_delay(Duration::from_secs(60))
    .with_max_retries(10);
```

### Jitter (Optional)

Add randomization to prevent thundering herd:

```rust
use async_retry::backoff::{ExponentialBackoff, Jitter};
use std::time::Duration;

let base_strategy = ExponentialBackoff::new(Duration::from_millis(100))
    .with_max_retries(5);

let strategy = Jitter::new(base_strategy);
```

## Advanced Usage

### Custom Backoff Strategy

Implement the `Backoff` trait (which is just an `Iterator<Item = Duration>`):

```rust
use async_retry::backoff::Backoff;
use std::time::Duration;

struct CustomBackoff {
    current: u64,
}

impl Iterator for CustomBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        self.current += 1;
        Some(Duration::from_secs(self.current))
    }
}

impl Backoff for CustomBackoff {}
```

## Examples

See the `examples/` directory for complete working examples:

- `simple_retry.rs`: Basic retry with exponential backoff
- `conditional_retry.rs`: Retry based on error type

Run an example:

```bash
cargo run --example simple_retry --features tokio-timer
```

## Runtime Support

This library is runtime-agnostic and supports:

- **Tokio**: Enable the `tokio-timer` feature
- **async-std**: Enable the `async-std-timer` feature

You must enable exactly one timer feature.

## Error Handling

All error types used with this library must implement:
- `std::fmt::Display` - for logging error messages
- `Send` - for thread safety in async contexts

For conditional retries, error types don't need to implement `std::error::Error`, but it's recommended for better error handling patterns.

## Performance Considerations

- The library uses boxed futures internally for flexibility
- All retry logic is lazy and doesn't allocate until needed
- Backoff strategies are iterators and can be chained with standard iterator adapters
- The `move` closure pattern ensures zero-cost abstractions for captured variables

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Acknowledgments

- Inspired by various retry libraries in the Rust ecosystem
- Built with asynchronous Rust best practices

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed history of changes.
