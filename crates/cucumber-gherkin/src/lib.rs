//! Cucumber/Gherkin testing framework implementation.
//!
//! Why?
//!
//! 1. It's straightforward to implement and easy to maintain ourselves.
//! 2. [cucumber-rs] is giving low confidence: low activity, a single primary maintainer, and a repository language breakdown of roughly 78% Rust, 18.8% JS, and 2.5% Gherkin.
//! 3. It'll be fun — building and owning a compact test runner is educational and enjoyable.
//!
//! [cucumber-rs]: https://github.com/cucumber-rs/cucumber
//!
#![allow(warnings)]
pub mod events;
pub mod gherkin;
pub mod parallel;
pub mod reporters;
pub mod results;
pub mod runner;
pub mod steps;
pub mod world;
pub mod world_runner;
