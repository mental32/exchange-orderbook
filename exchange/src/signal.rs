//! Signal handling.
//!
//! This module provides a [`Signals`] type that can be used to have
//! the application respond to signals such as SIGINT, SIGTERM, SIGQUIT, SIGHUP.    
//!
//! This is useful for graceful shutdowns, and for triggering signal behavior
//! from tests.
//!

use std::future::Future;

/// A signal source.
#[derive(Debug, Clone, Copy)]
enum SignalSource {
    /// use tokio's signal handling.
    Tokio,
}

/// A abstraction for signal handling.
#[derive(Debug, Clone, Copy)]
pub struct Signals {
    /// SIGINT signal source.
    ctrl_c: SignalSource,
}

impl Signals {
    /// Returns a future that resolves when a SIGINT is received.
    #[track_caller]
    #[must_use]
    pub fn ctrl_c(&self) -> impl Future<Output = Result<(), ()>> + '_ {
        match self.ctrl_c {
            SignalSource::Tokio => {
                let mut signal =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                        .expect("failed to register signal handler");

                async move {
                    match signal.recv().await {
                        Some(()) => Ok(()),
                        None => Err(()),
                    }
                }
            }
        }
    }
}

/// Returns a [`Signals`] instance that uses the host OS's signal handling.
pub fn from_host_os() -> Signals {
    Signals {
        ctrl_c: SignalSource::Tokio,
    }
}
