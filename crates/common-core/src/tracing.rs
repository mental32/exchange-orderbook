use tracing_subscriber::fmt::SubscriberBuilder;

/// general-purpose tracing subscriber with: json output, file, thread-ids, and line numbers
pub fn preconfigured_subscriber() -> SubscriberBuilder<
    tracing_subscriber::fmt::format::JsonFields,
    tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Json>,
    tracing_subscriber::EnvFilter,
> {
    tracing_subscriber::fmt::fmt()
        .with_file(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
}
