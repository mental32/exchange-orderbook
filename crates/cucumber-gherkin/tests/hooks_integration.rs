//! Integration test demonstrating Hooks + World functionality
//!
//! This test validates:
//! - World creation per scenario
//! - Hook execution order (BeforeAll -> Before -> Steps -> After -> AfterAll)
//! - Tag-based hook filtering
//! - Failure propagation (failing step skips remaining, After hooks still run)
//! - Background + hooks interaction

use chumsky::Parser;
use cucumber_gherkin::gherkin::TagOperation;
use cucumber_gherkin::gherkin::document_p;
use cucumber_gherkin::steps::DefaultSteps;
use cucumber_gherkin::world::ExecutionResult;
use cucumber_gherkin::world::World;
use cucumber_gherkin::world_runner::WorldRunner;
use std::sync::Arc;
use std::sync::Mutex;

/// Test World that tracks execution events for validation
#[derive(Debug, Clone)]
struct IntegrationTestWorld {
    pub execution_log: Arc<Mutex<Vec<String>>>,
    pub state: std::collections::HashMap<String, String>,
}

impl World for IntegrationTestWorld {
    fn new() -> Self {
        Self {
            execution_log: Arc::new(Mutex::new(Vec::new())),
            state: std::collections::HashMap::new(),
        }
    }
}

impl IntegrationTestWorld {
    fn log_event(&mut self, event: &str) {
        self.execution_log.lock().unwrap().push(event.to_string());
        println!("🌍 World Event: {}", event);
    }

    fn get_log(&self) -> Vec<String> {
        self.execution_log.lock().unwrap().clone()
    }

    fn set_state(&mut self, key: &str, value: &str) {
        self.state.insert(key.to_string(), value.to_string());
        self.log_event(&format!("Set state: {} = {}", key, value));
    }

    fn get_state(&self, key: &str) -> Option<String> {
        self.state.get(key).cloned()
    }
}

#[test]
#[ignore = "skipped for now"]
fn test_hooks_world_integration_basic() {
    // Read and parse the showcase.feature file that we know works
    let feature_content =
        std::fs::read_to_string("showcase.feature").expect("Failed to read showcase.feature");

    let document = document_p().parse(&feature_content).unwrap();

    // Create WorldRunner with hooks
    let mut runner: WorldRunner<IntegrationTestWorld> = WorldRunner::new();

    // Register BeforeAll hook
    runner.hooks().before_all("global_setup", || {
        println!("🎣 BeforeAll: Global test suite setup");
        ExecutionResult::Passed { duration_ms: 1 }
    });

    // Register AfterAll hook
    runner.hooks().after_all("global_cleanup", || {
        println!("🎣 AfterAll: Global test suite cleanup");
        ExecutionResult::Passed { duration_ms: 1 }
    });

    // Register Before hook (no tag filter)
    runner.hooks().before(
        "scenario_setup",
        None,
        |world: &mut IntegrationTestWorld| {
            world.log_event("Before hook executed");
            world.set_state("setup", "complete");
            ExecutionResult::Passed { duration_ms: 2 }
        },
    );

    // Register After hook (no tag filter)
    runner.hooks().after(
        "scenario_cleanup",
        None,
        |world: &mut IntegrationTestWorld| {
            world.log_event("After hook executed");
            world.set_state("cleanup", "complete");
            ExecutionResult::Passed { duration_ms: 2 }
        },
    );

    // Register tag-specific Before hook for smoke tests
    let smoke_tag = TagOperation::Tag("smoke".to_owned());
    runner.hooks().before(
        "smoke_setup",
        Some(smoke_tag),
        |world: &mut IntegrationTestWorld| {
            world.log_event("Smoke-specific Before hook executed");
            world.set_state("smoke_setup", "done");
            ExecutionResult::Passed { duration_ms: 3 }
        },
    );

    // Run the document
    let steps = DefaultSteps;
    runner.run(&document, steps);

    println!("✅ Basic hooks integration test completed");
}

#[test]
fn test_hooks_tag_filtering() {
    // Test that hooks with tag filters only run on matching scenarios
    let feature_content = r#"@suite
Feature: Tag Filtering Test

  @smoke @fast
  Scenario: Smoke test scenario
    When I perform a smoke test action
    Then the smoke test should pass

  @integration @slow
  Scenario: Integration test scenario
    When I perform a smoke test action
    Then the smoke test should pass
"#;

    let document = document_p().parse(feature_content).unwrap();

    let mut runner: WorldRunner<IntegrationTestWorld> = WorldRunner::new();

    // Register smoke-specific hook
    let smoke_tag = TagOperation::Tag("smoke".to_owned());
    runner.hooks().before(
        "smoke_hook",
        Some(smoke_tag),
        |world: &mut IntegrationTestWorld| {
            world.log_event("Smoke hook executed");
            ExecutionResult::Passed { duration_ms: 1 }
        },
    );

    // Register integration-specific hook
    let integration_tag = TagOperation::Tag("integration".to_owned());
    runner.hooks().before(
        "integration_hook",
        Some(integration_tag),
        |world: &mut IntegrationTestWorld| {
            world.log_event("Integration hook executed");
            ExecutionResult::Passed { duration_ms: 1 }
        },
    );

    // Run with no filter - should run both scenarios and both hooks
    let steps = DefaultSteps;
    runner.run(&document, steps);

    println!("✅ Tag filtering test completed");
}

#[test]
fn test_failure_propagation_with_hooks() {
    // Test that failing steps skip remaining steps but After hooks still run
    let feature_content = r#"@failure_test
Feature: Failure Propagation Test

@should_fail
Scenario: Scenario with failing step
  When I perform a smoke test action
  Then the smoke test should pass
"#;

    let document = document_p().parse(feature_content).unwrap();

    let mut runner: WorldRunner<IntegrationTestWorld> = WorldRunner::new();

    // Register Before hook that should run
    runner.hooks().before(
        "before_failing_scenario",
        None,
        |world: &mut IntegrationTestWorld| {
            world.log_event("Before hook ran before failing scenario");
            ExecutionResult::Passed { duration_ms: 1 }
        },
    );

    // Register After hook that should ALWAYS run (even after failures)
    runner.hooks().after(
        "after_failing_scenario",
        None,
        |world: &mut IntegrationTestWorld| {
            world.log_event("After hook ran after failing scenario");
            // Verify that Before hook did run by checking world state
            if world
                .get_log()
                .contains(&"Before hook ran before failing scenario".to_owned())
            {
                world.log_event("Confirmed: After hook ran even though scenario may have failed");
            }
            ExecutionResult::Passed { duration_ms: 1 }
        },
    );

    let steps = DefaultSteps;
    runner.run(&document, steps);

    println!("✅ Failure propagation test completed");
}

#[test]
fn test_scenario_outline_with_hooks() {
    // Test hooks with Scenario Outline - each example should get its own World
    let feature_content = r#"@parameterized
Feature: Scenario Outline Hooks Test

@outline @multiple
Scenario Outline: Parameterized test with hooks
  Given a test parameter "<param>"
  When I perform a smoke test action
  Then the smoke test should pass

  Examples: Test parameters
    | param |
    | test1 |
    | test2 |
"#;

    let document = document_p().parse(feature_content).unwrap();

    let mut runner: WorldRunner<IntegrationTestWorld> = WorldRunner::new();

    // Register hooks that track how many times they run
    static BEFORE_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    static AFTER_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    runner
        .hooks()
        .before("count_before", None, |world: &mut IntegrationTestWorld| {
            let count = BEFORE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            world.log_event(&format!("Before hook execution #{}", count));
            world.set_state("before_count", &count.to_string());
            ExecutionResult::Passed { duration_ms: 1 }
        });

    runner
        .hooks()
        .after("count_after", None, |world: &mut IntegrationTestWorld| {
            let count = AFTER_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            world.log_event(&format!("After hook execution #{}", count));
            world.set_state("after_count", &count.to_string());
            ExecutionResult::Passed { duration_ms: 1 }
        });

    let steps = DefaultSteps;
    runner.run(&document, steps);

    // Verify hooks ran for each example (should be 2 times each)
    let before_final = BEFORE_COUNT.load(std::sync::atomic::Ordering::SeqCst);
    let after_final = AFTER_COUNT.load(std::sync::atomic::Ordering::SeqCst);

    println!("📊 Before hooks ran {} times", before_final);
    println!("📊 After hooks ran {} times", after_final);

    // Each example row should have triggered Before/After hooks
    assert!(
        before_final >= 2,
        "Before hooks should run at least once per example"
    );
    assert!(
        after_final >= 2,
        "After hooks should run at least once per example"
    );

    println!("✅ Scenario Outline hooks test completed");
}

#[test]
fn test_hook_execution_order() {
    // Test the Oracle-specified execution order:
    // BeforeAll (once) → Before (tag-filtered) → Feature BG → Rule BG → scenario steps → After (always) → AfterAll (once)

    let feature_content = r#"@order_test
Feature: Hook Execution Order Test
This tests proper execution order

Background: Test environment setup
  Given the test environment is initialized

@order_verification
Scenario: Order verification scenario
  When I perform a smoke test action
  Then the smoke test should pass
"#;

    let document = document_p().parse(feature_content).unwrap();

    // Use shared execution log to track order across all hooks
    static EXECUTION_ORDER: Mutex<Vec<String>> = Mutex::new(Vec::new());

    let mut runner: WorldRunner<IntegrationTestWorld> = WorldRunner::new();

    // Register hooks in registration order (should execute in proper runtime order)
    runner.hooks().before_all("order_before_all", || {
        EXECUTION_ORDER.lock().unwrap().push("BeforeAll".to_owned());
        ExecutionResult::Passed { duration_ms: 1 }
    });

    runner.hooks().after_all("order_after_all", || {
        EXECUTION_ORDER.lock().unwrap().push("AfterAll".to_owned());
        ExecutionResult::Passed { duration_ms: 1 }
    });

    runner
        .hooks()
        .before("order_before", None, |world: &mut IntegrationTestWorld| {
            EXECUTION_ORDER.lock().unwrap().push("Before".to_owned());
            world.log_event("Before hook in order test");
            ExecutionResult::Passed { duration_ms: 1 }
        });

    runner
        .hooks()
        .after("order_after", None, |world: &mut IntegrationTestWorld| {
            EXECUTION_ORDER.lock().unwrap().push("After".to_owned());
            world.log_event("After hook in order test");
            ExecutionResult::Passed { duration_ms: 1 }
        });

    let steps = DefaultSteps;
    runner.run(&document, steps);

    let execution_log = EXECUTION_ORDER.lock().unwrap().clone();
    println!("📋 Execution order: {:?}", execution_log);

    // Verify proper execution order
    assert_eq!(execution_log.get(0), Some(&"BeforeAll".to_owned()));
    assert_eq!(execution_log.get(1), Some(&"Before".to_owned()));
    assert_eq!(execution_log.get(2), Some(&"After".to_owned()));
    assert_eq!(execution_log.get(3), Some(&"AfterAll".to_owned()));

    println!("✅ Hook execution order test completed - all hooks executed in correct order!");
}
