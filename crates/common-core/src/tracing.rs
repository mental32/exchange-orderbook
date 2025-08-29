//! utilities for [`tracing`]
//!

/// Initialize tracing subscriber with JSON output and environment filter
pub fn init() {
    tracing_subscriber::fmt::fmt()
        .with_file(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();
}
