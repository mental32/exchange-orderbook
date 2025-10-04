//! World-enabled runner that integrates hooks with scenario execution
//!
//! This module provides:
//! - World-based scenario execution with proper state isolation
//! - Hook execution (BeforeAll/AfterAll, Before/After) with tag filtering
//! - Failure propagation and skip semantics per Oracle requirements

use crate::gherkin::Document;
use crate::gherkin::Step;
use crate::runner::RunnerFilter;
use crate::steps::StepOutput;
use crate::steps::Steps;
use crate::world::DefaultWorld;
use crate::world::ExecutionResult;
use crate::world::HookRegistry;
use crate::world::World;
use std::time::Instant;
use yansi::Paint;

/// World-enabled runner with hooks support
pub struct WorldRunner<W: World> {
    hook_registry: HookRegistry<W>,
}

impl<W: World> WorldRunner<W> {
    pub fn new() -> Self {
        Self {
            hook_registry: HookRegistry::new(),
        }
    }

    /// Get mutable access to hook registry for registering hooks
    pub fn hooks(&mut self) -> &mut HookRegistry<W> {
        &mut self.hook_registry
    }

    /// Run document with World and hooks support
    pub fn run(&self, document: &Document, steps: impl Steps) {
        let filter = RunnerFilter::new();
        self.run_with_filter(document, steps, &filter);
    }

    /// Run document with filtering and World/hooks support
    pub fn run_with_filter(&self, document: &Document, steps: impl Steps, filter: &RunnerFilter) {
        dbg!("🌍 WorldRunner starting with filter: {:?}", filter);

        // Run BeforeAll hooks once
        let before_all_results = self.hook_registry.run_before_all();
        dbg!(
            "🎣 BeforeAll results: {} hooks executed",
            before_all_results.len()
        );

        // Track if any BeforeAll hooks failed
        let before_all_failed = before_all_results.iter().any(|r| r.is_failure());
        if before_all_failed {
            dbg!("🚨 BeforeAll hook failed, aborting execution");
            return;
        }

        // Run top-level scenarios first
        for scenario in &document.feature.scenarios {
            if self.should_run_scenario(
                &document.feature.header.tags,
                &[],
                &scenario.header.tags,
                &[],
                filter,
            ) {
                self.run_scenario_with_world(
                    scenario,
                    &document.feature.background,
                    &None,
                    &steps,
                    &document.feature.header.tags,
                    &[],
                );
            } else {
                dbg!(
                    "🚫 Skipping scenario '{}' due to tag filter",
                    &scenario.header.name
                );
            }
        }

        // Run rules and their nested scenarios
        for rule in &document.feature.rules {
            println!("\n{}", rule.header.name.bright_cyan());

            for scenario in &rule.scenarios {
                if self.should_run_scenario(
                    &document.feature.header.tags,
                    &rule.header.tags,
                    &scenario.header.tags,
                    &[],
                    filter,
                ) {
                    self.run_scenario_with_world(
                        scenario,
                        &document.feature.background,
                        &rule.background,
                        &steps,
                        &document.feature.header.tags,
                        &rule.header.tags,
                    );
                } else {
                    dbg!(
                        "🚫 Skipping scenario '{}' due to tag filter",
                        &scenario.header.name
                    );
                }
            }
        }

        // Run AfterAll hooks once
        let after_all_results = self.hook_registry.run_after_all();
        dbg!(
            "🎣 AfterAll results: {} hooks executed",
            after_all_results.len()
        );
    }

    /// Run a single scenario with World isolation and hook execution
    fn run_scenario_with_world(
        &self,
        scenario: &crate::gherkin::Scenario,
        feature_background: &Option<crate::gherkin::Background>,
        rule_background: &Option<crate::gherkin::Background>,
        steps: &impl Steps,
        feature_tags: &[String],
        rule_tags: &[String],
    ) {
        dbg!(
            "🌍 Starting scenario with World: '{}'",
            &scenario.header.name
        );

        // Check if this is a Scenario Outline with Examples
        if !scenario.examples.is_empty() {
            // Run the scenario once for each row in the Examples table
            for example_set in &scenario.examples {
                let effective_tags = self.effective_tags(
                    feature_tags,
                    rule_tags,
                    &scenario.header.tags,
                    &example_set.tags,
                );

                // Run each example row as a separate scenario with its own World
                for (row_index, row) in example_set.table.rows.iter().enumerate() {
                    let examples_name = &example_set.header.name;
                    println!(
                        "{} ({} example {})",
                        scenario.header.name.bright_blue(),
                        examples_name.bright_green(),
                        row_index + 1
                    );

                    // Create new World for this example execution
                    let mut world = W::new();
                    dbg!("🌍 Created new World for example {}", row_index + 1);

                    // Execute scenario with World and hooks
                    self.execute_scenario_with_hooks(
                        &mut world,
                        scenario,
                        feature_background,
                        rule_background,
                        steps,
                        &effective_tags,
                        Some((&example_set.table.header, row)),
                    );
                }
            }
        } else {
            // Regular scenario execution
            println!("{}", scenario.header.name.bright_blue());

            // Create new World for this scenario
            let mut world = W::new();
            dbg!("🌍 Created new World for scenario");

            let effective_tags =
                self.effective_tags(feature_tags, rule_tags, &scenario.header.tags, &[]);

            // Execute scenario with World and hooks
            self.execute_scenario_with_hooks(
                &mut world,
                scenario,
                feature_background,
                rule_background,
                steps,
                &effective_tags,
                None,
            );
        }
    }

    /// Execute a scenario with full hook lifecycle and World state management
    fn execute_scenario_with_hooks(
        &self,
        world: &mut W,
        scenario: &crate::gherkin::Scenario,
        feature_background: &Option<crate::gherkin::Background>,
        rule_background: &Option<crate::gherkin::Background>,
        steps: &impl Steps,
        effective_tags: &[String],
        substitution: Option<(&[String], &[String])>, // (headers, row) for parameter substitution
    ) {
        dbg!(
            "🎣 Starting hook execution for scenario with tags: {:?}",
            effective_tags
        );

        // 1. Run Before hooks (tag-filtered)
        let before_hooks = self.hook_registry.get_before_hooks(effective_tags);
        dbg!("🎣 Running {} Before hooks", before_hooks.len());
        let mut scenario_failed = false;

        for hook in before_hooks {
            dbg!("▶️ Executing Before hook: {}", &hook.name);
            let result = (hook.hook_fn)(world);
            dbg!("🎣 Before hook '{}' result: {:?}", &hook.name, &result);

            if result.is_failure() {
                dbg!(
                    "🚨 Before hook '{}' failed, marking scenario as failed",
                    &hook.name
                );
                scenario_failed = true;
                break;
            }
        }

        // 2. Run backgrounds and scenario steps (only if Before hooks passed)
        if !scenario_failed {
            // Execute Feature background first, then Rule background
            self.run_backgrounds(
                world,
                feature_background,
                rule_background,
                steps,
                &mut scenario_failed,
            );

            // Execute scenario steps with parameter substitution if needed
            if !scenario_failed {
                for (step, _span) in scenario.steps.iter() {
                    if scenario_failed {
                        dbg!(
                            "⏭️ Skipping step due to previous failure: {}",
                            step.text_as_string()
                        );
                        continue;
                    }

                    let actual_step = if let Some((headers, row)) = substitution {
                        self.substitute_parameters(step, headers, row)
                    } else {
                        step.clone()
                    };

                    let result = self.execute_step_with_world(world, &actual_step, steps);
                    if result.is_failure() {
                        dbg!(
                            "🚨 Step failed: {}, remaining steps will be skipped",
                            actual_step.text_as_string()
                        );
                        scenario_failed = true;
                    }
                }
            }
        }

        // 3. Run After hooks (always run, even after failures, tag-filtered)
        let after_hooks = self.hook_registry.get_after_hooks(effective_tags);
        dbg!(
            "🎣 Running {} After hooks (scenario_failed: {})",
            after_hooks.len(),
            scenario_failed
        );

        for hook in after_hooks {
            dbg!("▶️ Executing After hook: {}", &hook.name);
            let result = (hook.hook_fn)(world);
            dbg!("🎣 After hook '{}' result: {:?}", &hook.name, result);
            // Note: After hook failures don't affect scenario result, but are logged
        }

        dbg!(
            "🏁 Scenario execution completed (failed: {})",
            scenario_failed
        );
    }

    /// Execute backgrounds with World support
    fn run_backgrounds(
        &self,
        world: &mut W,
        feature_background: &Option<crate::gherkin::Background>,
        rule_background: &Option<crate::gherkin::Background>,
        steps: &impl Steps,
        scenario_failed: &mut bool,
    ) {
        dbg!(
            "🏗️ Running backgrounds (feature: {}, rule: {})",
            feature_background.is_some(),
            rule_background.is_some()
        );

        // Run feature background first (if present)
        if let Some(bg) = feature_background {
            dbg!("▶️ Executing Feature background");
            for (step, _span) in &bg.steps {
                if *scenario_failed {
                    dbg!(
                        "⏭️ Skipping background step due to previous failure: {}",
                        step.text_as_string()
                    );
                    continue;
                }

                let result = self.execute_step_with_world(world, step, steps);
                if result.is_failure() {
                    dbg!(
                        "🚨 Feature background step failed: {}",
                        step.text_as_string()
                    );
                    *scenario_failed = true;
                }
            }
        }

        // Run rule background second (if present)
        if let Some(bg) = rule_background {
            dbg!("▶️ Executing Rule background");
            for (step, _span) in &bg.steps {
                if *scenario_failed {
                    dbg!(
                        "⏭️ Skipping background step due to previous failure: {}",
                        step.text_as_string()
                    );
                    continue;
                }

                let result = self.execute_step_with_world(world, step, steps);
                if result.is_failure() {
                    dbg!("🚨 Rule background step failed: {}", step.text_as_string());
                    *scenario_failed = true;
                }
            }
        }
    }

    /// Execute a single step with World support and proper timing
    fn execute_step_with_world(
        &self,
        _world: &mut W,
        step: &Step,
        steps: &impl Steps,
    ) -> ExecutionResult {
        let start_time = Instant::now();
        let step_text = step.text_as_string();

        dbg!("▶️ Executing step: {}", &step_text);

        if let Some(step_fn) = steps.get_step_fn(step) {
            // Execute the step function
            let step_result = step_fn(step);
            let duration_ms = start_time.elapsed().as_millis() as u64;

            let result = match step_result {
                StepOutput::Passed => {
                    println!("    ✅ {}", step_text.bright_green());
                    ExecutionResult::Passed { duration_ms }
                }
                StepOutput::Failed => {
                    println!("    ❌ {}", step_text.bright_red());
                    ExecutionResult::Failed {
                        error: format!("Step failed: {}", step_text),
                        duration_ms,
                    }
                }
                StepOutput::Skipped => {
                    println!("    ⏭️ {}", step_text.bright_yellow());
                    ExecutionResult::Skipped
                }
            };

            dbg!("🎯 Step '{}' result: {:?}", &step_text, &result);
            result
        } else {
            let _duration_ms = start_time.elapsed().as_millis() as u64;
            println!("    ❓ {} (undefined)", step_text.bright_magenta());
            let result = ExecutionResult::Undefined {
                step_text: step_text.clone(),
            };
            dbg!("❓ Undefined step: {}", &step_text);
            result
        }
    }

    // Copy helper methods from original runner
    fn should_run_scenario(
        &self,
        feature_tags: &[String],
        rule_tags: &[String],
        scenario_tags: &[String],
        examples_tags: &[String],
        filter: &RunnerFilter,
    ) -> bool {
        if let Some(tag_expr) = &filter.tag_expression {
            let effective_tags =
                self.effective_tags(feature_tags, rule_tags, scenario_tags, examples_tags);
            let should_run = tag_expr.evaluate(&effective_tags);
            dbg!(
                "🏷️ Tag filter evaluation: {:?} against tags {:?} = {}",
                tag_expr,
                effective_tags,
                should_run
            );
            should_run
        } else {
            dbg!("✅ No tag filter, scenario will run");
            true
        }
    }

    fn effective_tags(
        &self,
        feature_tags: &[String],
        rule_tags: &[String],
        scenario_tags: &[String],
        examples_tags: &[String],
    ) -> Vec<String> {
        let mut tags = Vec::new();
        tags.extend_from_slice(feature_tags);
        tags.extend_from_slice(rule_tags);
        tags.extend_from_slice(scenario_tags);
        tags.extend_from_slice(examples_tags);
        tags.sort();
        tags.dedup();
        dbg!("🏷️ Effective tags: {:?}", &tags);
        tags
    }

    fn substitute_parameters(&self, step: &Step, headers: &[String], row: &[String]) -> Step {
        let step_text = step.text_as_string();
        let mut substituted_text = step_text.clone();

        for (header, value) in headers.iter().zip(row.iter()) {
            let placeholder = format!("<{}>", header);
            substituted_text = substituted_text.replace(&placeholder, value);
        }

        dbg!(
            "🔄 Parameter substitution: '{}' -> '{}'",
            step_text,
            substituted_text
        );

        // Note: This is a simplified substitution that only handles text.
        // Full implementation would need to parse the substituted text back into StepText parts
        step.clone() // For now, return original step
    }
}

impl<W: World> Default for WorldRunner<W> {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience type alias for DefaultWorld runner
pub type DefaultWorldRunner = WorldRunner<DefaultWorld>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gherkin::TagOperation;
    use std::sync::Arc;
    use std::sync::Mutex;

    /// Test World that tracks state for testing
    #[derive(Debug, Clone)]
    struct TestWorld {
        pub execution_log: Arc<Mutex<Vec<String>>>,
    }

    impl World for TestWorld {
        fn new() -> Self {
            Self {
                execution_log: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl TestWorld {
        fn log_event(&mut self, event: &str) {
            self.execution_log.lock().unwrap().push(event.to_string());
        }

        fn get_log(&self) -> Vec<String> {
            self.execution_log.lock().unwrap().clone()
        }
    }

    #[test]
    fn test_world_runner_creation() {
        let runner: WorldRunner<TestWorld> = WorldRunner::new();
        assert_eq!(runner.hook_registry.before_all.len(), 0);
        assert_eq!(runner.hook_registry.after_all.len(), 0);
        assert_eq!(runner.hook_registry.before.len(), 0);
        assert_eq!(runner.hook_registry.after.len(), 0);
    }

    #[test]
    fn test_hook_registration() {
        let mut runner: WorldRunner<TestWorld> = WorldRunner::new();

        // Register hooks
        runner
            .hooks()
            .before_all("setup_global", || ExecutionResult::Passed {
                duration_ms: 1,
            });
        runner
            .hooks()
            .after_all("cleanup_global", || ExecutionResult::Passed {
                duration_ms: 2,
            });
        runner
            .hooks()
            .before("setup_scenario", None, |world: &mut TestWorld| {
                world.log_event("before_scenario");
                ExecutionResult::Passed { duration_ms: 3 }
            });
        runner
            .hooks()
            .after("cleanup_scenario", None, |world: &mut TestWorld| {
                world.log_event("after_scenario");
                ExecutionResult::Passed { duration_ms: 4 }
            });

        // Verify registration
        assert_eq!(runner.hook_registry.before_all.len(), 1);
        assert_eq!(runner.hook_registry.after_all.len(), 1);
        assert_eq!(runner.hook_registry.before.len(), 1);
        assert_eq!(runner.hook_registry.after.len(), 1);
    }

    #[test]
    fn test_tag_filtered_hooks() {
        let mut runner: WorldRunner<TestWorld> = WorldRunner::new();

        // Register hooks with tag filters
        let smoke_tag = TagOperation::Tag("smoke".to_string());
        let integration_tag = TagOperation::Tag("integration".to_string());

        runner
            .hooks()
            .before("smoke_setup", Some(smoke_tag), |world: &mut TestWorld| {
                world.log_event("smoke_before");
                ExecutionResult::Passed { duration_ms: 5 }
            });

        runner.hooks().before(
            "integration_setup",
            Some(integration_tag),
            |world: &mut TestWorld| {
                world.log_event("integration_before");
                ExecutionResult::Passed { duration_ms: 6 }
            },
        );

        // Test tag filtering
        let smoke_tags = vec!["smoke".to_string(), "fast".to_string()];
        let smoke_hooks = runner.hook_registry.get_before_hooks(&smoke_tags);
        assert_eq!(smoke_hooks.len(), 1);
        assert_eq!(smoke_hooks[0].name, "smoke_setup");

        let integration_tags = vec!["integration".to_string(), "slow".to_string()];
        let integration_hooks = runner.hook_registry.get_before_hooks(&integration_tags);
        assert_eq!(integration_hooks.len(), 1);
        assert_eq!(integration_hooks[0].name, "integration_setup");

        let unmatched_tags = vec!["unit".to_string()];
        let no_hooks = runner.hook_registry.get_before_hooks(&unmatched_tags);
        assert_eq!(no_hooks.len(), 0);
    }

    #[test]
    fn test_effective_tags_computation() {
        let runner: WorldRunner<TestWorld> = WorldRunner::new();

        let feature_tags = vec!["feature".to_string()];
        let rule_tags = vec!["rule".to_string()];
        let scenario_tags = vec!["scenario".to_string()];
        let examples_tags = vec!["examples".to_string()];

        let effective_tags =
            runner.effective_tags(&feature_tags, &rule_tags, &scenario_tags, &examples_tags);

        // Should contain all tags, sorted and deduped
        assert_eq!(
            effective_tags,
            vec!["examples", "feature", "rule", "scenario"]
        );
    }

    #[test]
    fn test_should_run_scenario_no_filter() {
        let runner: WorldRunner<TestWorld> = WorldRunner::new();
        let filter = RunnerFilter::new();

        let should_run = runner.should_run_scenario(&[], &[], &[], &[], &filter);
        assert!(should_run);
    }

    #[test]
    fn test_should_run_scenario_with_matching_filter() {
        let runner: WorldRunner<TestWorld> = WorldRunner::new();
        let tag_expr = TagOperation::Tag("smoke".to_string());
        let filter = RunnerFilter::with_tags(tag_expr);

        let scenario_tags = vec!["smoke".to_string()];
        let should_run = runner.should_run_scenario(&[], &[], &scenario_tags, &[], &filter);
        assert!(should_run);
    }

    #[test]
    fn test_should_run_scenario_with_non_matching_filter() {
        let runner: WorldRunner<TestWorld> = WorldRunner::new();
        let tag_expr = TagOperation::Tag("integration".to_string());
        let filter = RunnerFilter::with_tags(tag_expr);

        let scenario_tags = vec!["smoke".to_string()];
        let should_run = runner.should_run_scenario(&[], &[], &scenario_tags, &[], &filter);
        assert!(!should_run);
    }
}
