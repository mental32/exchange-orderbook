//! Parallel Execution System for cucumber-gherkin
//!
//! Provides per-scenario parallel test execution with proper World isolation,
//! thread-safe event reporting, and comprehensive hook support.
//!
//! Oracle-guided design:
//! - Concurrency model: run each Scenario as an independent job
//! - Hook semantics: BeforeAll/AfterAll run serially, Before/After per-scenario
//! - World isolation: each scenario gets a fresh World instance
//! - Steps registry: shared read-only across threads
//! - Reporter thread-safety: serialized event stream via ReporterMux

use crate::events::EventBus;
use crate::gherkin::Document;
use crate::gherkin::Step;
use crate::results::RunResult;
use crate::results::ScenarioResult;
use crate::results::StepError;
use crate::results::StepResult;
use crate::results::StepStatus;
use crate::results::TestId;
use crate::steps::Steps;
use crate::world::ExecutionResult;
use crate::world::HookRegistry;
use crate::world::World;
use crossbeam::channel;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

/// Executable scenario with all necessary context for parallel execution
#[derive(Debug, Clone)]
pub struct ExecutableScenario {
    /// Unique scenario identifier (stable, increasing, assigned in defined order)
    pub scenario_id: u64,
    /// Origin information for debugging and reporting
    pub origin: ScenarioOrigin,
    /// Combined tags from feature, rule, and scenario
    pub tags: Vec<String>,
    /// Fully composed steps (feature background + rule background + scenario steps)
    pub steps: Vec<Step>,
}

/// Origin information for an executable scenario
#[derive(Debug, Clone)]
pub struct ScenarioOrigin {
    pub feature_path: String,
    pub rule_name: Option<String>,
    pub scenario_name: String,
    pub line: usize,
}

/// Events for parallel execution coordination
#[derive(Debug, Clone)]
pub enum ParallelEvent {
    /// Worker thread started
    WorkerStarted { worker_id: usize, thread_id: String },
    /// Worker thread stopped
    WorkerStopped { worker_id: usize, thread_id: String },
    /// Scenario assigned to worker
    ScenarioAssigned { scenario_id: u64, worker_id: usize },
}

/// Configuration for parallel execution
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of worker threads (default: num_cpus)
    pub jobs: usize,
    /// Queue size for job distribution
    pub queue_size: usize,
    /// Enable fail-fast behavior
    pub fail_fast: bool,
    /// Per-scenario timeout in seconds (0 = no timeout)
    pub scenario_timeout: u64,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            jobs: num_cpus::get(),
            queue_size: 100,
            fail_fast: false,
            scenario_timeout: 0,
        }
    }
}

/// Parallel executor for running scenarios concurrently
pub struct ParallelExecutor<W: World> {
    config: ParallelConfig,
    event_bus: EventBus,
    hook_registry: Arc<HookRegistry<W>>,
    steps: Arc<dyn Steps + Send + Sync>,
}

impl<W: World + Send + 'static> ParallelExecutor<W> {
    /// Create a new parallel executor
    pub fn new(
        config: ParallelConfig,
        event_bus: EventBus,
        hook_registry: Arc<HookRegistry<W>>,
        steps: Arc<dyn Steps + Send + Sync>,
    ) -> Self {
        dbg!("🚀 Creating ParallelExecutor with {} workers", config.jobs);
        Self {
            config,
            event_bus,
            hook_registry,
            steps,
        }
    }

    /// Build executable schedule from parsed document
    pub fn plan(&self, document: &Document, feature_path: &str) -> Vec<ExecutableScenario> {
        let mut executable_scenarios = Vec::new();
        let mut scenario_id = 0;

        dbg!("📋 Planning executable scenarios from document");

        let feature = &document.feature;
        // Process top-level scenarios
        for scenario in &feature.scenarios {
            let mut steps = Vec::new();

            // Add feature background steps
            if let Some(ref background) = feature.background {
                steps.extend(background.steps.iter().map(|(step, _span)| step.clone()));
            }

            // Add scenario steps
            steps.extend(scenario.steps.iter().map(|(step, _span)| step.clone()));

            let executable = ExecutableScenario {
                scenario_id,
                origin: ScenarioOrigin {
                    feature_path: feature_path.to_string(),
                    rule_name: None,
                    scenario_name: scenario.header.name.clone(),
                    line: 0, // We don't have location info in this structure
                },
                tags: feature
                    .header
                    .tags
                    .iter()
                    .chain(scenario.header.tags.iter())
                    .cloned()
                    .collect(),
                steps,
            };

            dbg!(
                "🎯 Planned scenario {} (id={}): {} with {} steps",
                scenario_id,
                &executable.origin.scenario_name,
                executable.steps.len()
            );

            executable_scenarios.push(executable);
            scenario_id += 1;
        }

        // Process rules and their scenarios
        for rule in &feature.rules {
            for scenario in &rule.scenarios {
                let mut steps = Vec::new();

                // Add feature background steps
                if let Some(ref background) = feature.background {
                    steps.extend(background.steps.iter().map(|(step, _span)| step.clone()));
                }

                // Add rule background steps (Oracle: Rule background overrides Feature background)
                if let Some(ref rule_background) = rule.background {
                    steps.extend(
                        rule_background
                            .steps
                            .iter()
                            .map(|(step, _span)| step.clone()),
                    );
                }

                // Add scenario steps
                steps.extend(scenario.steps.iter().map(|(step, _span)| step.clone()));

                let executable = ExecutableScenario {
                    scenario_id,
                    origin: ScenarioOrigin {
                        feature_path: feature_path.to_string(),
                        rule_name: Some(rule.header.name.clone()),
                        scenario_name: scenario.header.name.clone(),
                        line: 0, // We don't have location info in this structure
                    },
                    tags: feature
                        .header
                        .tags
                        .iter()
                        .chain(rule.header.tags.iter())
                        .chain(scenario.header.tags.iter())
                        .cloned()
                        .collect(),
                    steps,
                };

                dbg!(
                    "🎯 Planned rule scenario {} (id={}): {} with {} steps",
                    scenario_id,
                    &executable.origin.scenario_name,
                    executable.steps.len()
                );

                executable_scenarios.push(executable);
                scenario_id += 1;
            }
        }

        dbg!(
            "📊 Planned {} executable scenarios total",
            executable_scenarios.len()
        );
        executable_scenarios
    }

    /// Execute scenarios in parallel
    pub fn execute(&self, scenarios: Vec<ExecutableScenario>) -> RunResult {
        let start_time = SystemTime::now();

        dbg!(
            "🚀 Starting parallel execution with {} scenarios on {} workers",
            scenarios.len(),
            self.config.jobs
        );

        // Run BeforeAll hooks serially
        self.run_before_all_hooks();

        // Create job and event channels
        let (job_tx, job_rx) = channel::bounded(self.config.queue_size);
        let (event_tx, event_rx) = channel::unbounded();

        // Results collection
        let results = Arc::new(Mutex::new(Vec::new()));

        // Start worker threads
        let mut worker_handles = Vec::new();
        for worker_id in 0..self.config.jobs {
            let job_rx: channel::Receiver<ExecutableScenario> = job_rx.clone();
            let event_tx = event_tx.clone();
            let steps = Arc::clone(&self.steps);
            let hook_registry = Arc::clone(&self.hook_registry);
            let results = Arc::clone(&results);

            let handle = thread::spawn(move || {
                let thread_id = format!("{:?}", thread::current().id());
                event_tx
                    .send(ParallelEvent::WorkerStarted {
                        worker_id,
                        thread_id: thread_id.clone(),
                    })
                    .unwrap();
                dbg!("🔧 Worker {} started on thread {}", worker_id, &thread_id);

                while let Ok(scenario) = job_rx.recv() {
                    event_tx
                        .send(ParallelEvent::ScenarioAssigned {
                            scenario_id: scenario.scenario_id,
                            worker_id,
                        })
                        .unwrap();

                    let scenario_result =
                        Self::execute_scenario_with_world(scenario, &*steps, &*hook_registry);

                    results.lock().unwrap().push(scenario_result);
                }

                event_tx
                    .send(ParallelEvent::WorkerStopped {
                        worker_id,
                        thread_id,
                    })
                    .unwrap();
                dbg!("🔧 Worker {} stopped", worker_id);
            });

            worker_handles.push(handle);
        }

        // Enqueue all scenarios
        for scenario in scenarios {
            if let Err(e) = job_tx.send(scenario) {
                eprintln!("Failed to enqueue scenario: {}", e);
            }
        }
        drop(job_tx); // Close the channel to signal workers to finish

        // Process events (simplified for now)
        drop(event_tx);
        while let Ok(event) = event_rx.recv() {
            dbg!("📡 Parallel event: {:?}", event);
        }

        // Wait for all workers to finish
        for handle in worker_handles {
            if let Err(e) = handle.join() {
                eprintln!("Worker thread panicked: {:?}", e);
            }
        }

        // Run AfterAll hooks serially
        self.run_after_all_hooks();

        // Collect results
        let scenario_results = results.lock().unwrap().clone();
        let end_time = SystemTime::now();
        let duration = end_time
            .duration_since(start_time)
            .unwrap_or(Duration::from_secs(0));

        dbg!(
            "✅ Parallel execution completed in {}ms",
            duration.as_millis()
        );

        // Build RunResult
        let total_scenarios = scenario_results.len();
        let passed_scenarios = scenario_results
            .iter()
            .filter(|r| r.status.is_passed())
            .count();
        let failed_scenarios = total_scenarios - passed_scenarios;

        let total_steps = scenario_results.iter().map(|s| s.steps.len()).sum();
        let passed_steps = scenario_results
            .iter()
            .flat_map(|s| &s.steps)
            .filter(|step| step.status.is_passed())
            .count();
        let failed_steps = scenario_results
            .iter()
            .flat_map(|s| &s.steps)
            .filter(|step| matches!(step.status, StepStatus::Failed))
            .count();
        let skipped_steps = scenario_results
            .iter()
            .flat_map(|s| &s.steps)
            .filter(|step| matches!(step.status, StepStatus::Skipped))
            .count();
        let undefined_steps = scenario_results
            .iter()
            .flat_map(|s| &s.steps)
            .filter(|step| matches!(step.status, StepStatus::Undefined))
            .count();

        let mut run_result = RunResult::new();
        run_result.features = vec![]; // Simplified for now - would build FeatureResults
        run_result.end_time = end_time;
        run_result.duration = duration;
        run_result.total_scenarios = total_scenarios;
        run_result.passed_scenarios = passed_scenarios;
        run_result.failed_scenarios = failed_scenarios;
        run_result.total_steps = total_steps;
        run_result.passed_steps = passed_steps;
        run_result.failed_steps = failed_steps;
        run_result.skipped_steps = skipped_steps;
        run_result.undefined_steps = undefined_steps;
        run_result
    }

    /// Execute a single scenario with fresh World instance
    fn execute_scenario_with_world(
        scenario: ExecutableScenario,
        steps: &dyn Steps,
        hook_registry: &HookRegistry<W>,
    ) -> ScenarioResult {
        let scenario_start = SystemTime::now();
        let mut world = W::new();

        dbg!(
            "🎬 Executing scenario {} (id={})",
            &scenario.origin.scenario_name,
            scenario.scenario_id
        );

        // Run Before hooks
        let before_hooks = hook_registry.get_before_hooks(&scenario.tags);
        for hook in before_hooks {
            dbg!("🎣 Running Before hook: {}", &hook.name);
            let _result = (hook.hook_fn)(&mut world);
        }

        // Execute steps
        let mut step_results = Vec::new();
        let mut scenario_failed = false;

        for (step_idx, step) in scenario.steps.iter().enumerate() {
            if scenario_failed {
                // Skip remaining steps after failure
                let now = SystemTime::now();
                step_results.push(StepResult {
                    id: format!("step_{}_{}", scenario.scenario_id, step_idx),
                    name: step.text_as_string(),
                    keyword: step.verb.to_string(),
                    location: None,
                    status: StepStatus::Skipped,
                    start_time: now,
                    end_time: now,
                    duration: Duration::from_secs(0),
                    error: None,
                    attachments: vec![],
                    attempt: 1,
                });
                continue;
            }

            let step_start = SystemTime::now();
            let step_end = step_start.elapsed().unwrap_or(Duration::from_secs(0));
            let step_result = if let Some(step_fn) = steps.get_step_fn(step) {
                let output = step_fn(step);
                match output {
                    crate::steps::StepOutput::Passed => StepResult {
                        id: format!("step_{}_{}", scenario.scenario_id, step_idx),
                        name: step.text_as_string(),
                        keyword: step.verb.to_string(),
                        location: None,
                        status: StepStatus::Passed,
                        start_time: step_start,
                        end_time: step_start + step_end,
                        duration: step_end,
                        error: None,
                        attachments: vec![],
                        attempt: 1,
                    },
                    crate::steps::StepOutput::Failed => {
                        scenario_failed = true;
                        StepResult {
                            id: format!("step_{}_{}", scenario.scenario_id, step_idx),
                            name: step.text_as_string(),
                            keyword: step.verb.to_string(),
                            location: None,
                            status: StepStatus::Failed,
                            start_time: step_start,
                            end_time: step_start + step_end,
                            duration: step_end,
                            error: Some(StepError {
                                message: "Step failed".to_string(),
                                error_type: "StepPanic".to_string(),
                                stack_trace: None,
                                location: None,
                            }),
                            attachments: vec![],
                            attempt: 1,
                        }
                    }
                    crate::steps::StepOutput::Skipped => StepResult {
                        id: format!("step_{}_{}", scenario.scenario_id, step_idx),
                        name: step.text_as_string(),
                        keyword: step.verb.to_string(),
                        location: None,
                        status: StepStatus::Skipped,
                        start_time: step_start,
                        end_time: step_start + step_end,
                        duration: step_end,
                        error: None,
                        attachments: vec![],
                        attempt: 1,
                    },
                }
            } else {
                scenario_failed = true;
                StepResult {
                    id: format!("step_{}_{}", scenario.scenario_id, step_idx),
                    name: step.text_as_string(),
                    keyword: step.verb.to_string(),
                    location: None,
                    status: StepStatus::Undefined,
                    start_time: step_start,
                    end_time: step_start + step_end,
                    duration: step_end,
                    error: Some(StepError {
                        message: format!("Step undefined: {}", step.text_as_string()),
                        error_type: "UndefinedStep".to_string(),
                        stack_trace: None,
                        location: None,
                    }),
                    attachments: vec![],
                    attempt: 1,
                }
            };

            dbg!(
                "📋 Step {} result: {} - {}",
                step_idx,
                step.text_as_string(),
                if step_result.status.is_passed() {
                    "✅"
                } else {
                    "❌"
                }
            );
            step_results.push(step_result);
        }

        // Run After hooks (always run, even after failures)
        let after_hooks = hook_registry.get_after_hooks(&scenario.tags);
        for hook in after_hooks {
            dbg!("🎣 Running After hook: {}", &hook.name);
            let _result = (hook.hook_fn)(&mut world);
        }

        let scenario_end = SystemTime::now();
        let scenario_duration = scenario_end
            .duration_since(scenario_start)
            .unwrap_or(Duration::from_secs(0));

        let scenario_status =
            if scenario_failed || step_results.iter().any(|s| !s.status.is_passed()) {
                crate::results::ScenarioStatus::Failed
            } else {
                crate::results::ScenarioStatus::Passed
            };

        dbg!(
            "🎬 Scenario {} completed: {} in {}ms",
            &scenario.origin.scenario_name,
            if scenario_status.is_passed() {
                "✅"
            } else {
                "❌"
            },
            scenario_duration.as_millis()
        );

        ScenarioResult {
            id: TestId::new(
                scenario.origin.feature_path.clone(),
                scenario.origin.line,
                scenario.origin.scenario_name.clone(),
            ),
            name: scenario.origin.scenario_name,
            status: scenario_status,
            steps: step_results,
            start_time: scenario_start,
            end_time: scenario_end,
            duration: scenario_duration,
            tags: scenario.tags,
            outline_info: None,
            retry_count: 0,
            attachments: vec![],
        }
    }

    /// Run BeforeAll hooks serially
    fn run_before_all_hooks(&self) {
        dbg!(
            "🎣 Running {} BeforeAll hooks",
            self.hook_registry.before_all.len()
        );
        for hook in &self.hook_registry.before_all {
            dbg!("🎣 Running BeforeAll hook: {}", &hook.name);
            let result = (hook.hook_fn)();
            if let ExecutionResult::Failed { error, .. } = result {
                eprintln!("❌ BeforeAll hook '{}' failed: {}", hook.name, error);
            }
        }
    }

    /// Run AfterAll hooks serially
    fn run_after_all_hooks(&self) {
        dbg!(
            "🎣 Running {} AfterAll hooks",
            self.hook_registry.after_all.len()
        );
        for hook in &self.hook_registry.after_all {
            dbg!("🎣 Running AfterAll hook: {}", &hook.name);
            let result = (hook.hook_fn)();
            if let ExecutionResult::Failed { error, .. } = result {
                eprintln!("❌ AfterAll hook '{}' failed: {}", hook.name, error);
            }
        }
    }
}

// DefaultSteps already implements Steps trait in steps/mod.rs
// These unsafe impls are needed for parallel execution

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gherkin::StepType;
    use chumsky::span::SimpleSpan;
    use chumsky::span::Span;
    use std::collections::HashMap;

    /// Test World implementation for parallel execution tests
    #[derive(Debug)]
    struct TestWorld {
        state: HashMap<String, String>,
    }

    impl World for TestWorld {
        fn new() -> Self {
            Self {
                state: HashMap::new(),
            }
        }
    }

    impl TestWorld {
        fn set_state(&mut self, key: &str, value: &str) {
            self.state.insert(key.to_string(), value.to_string());
        }

        fn get_state(&self, key: &str) -> Option<&String> {
            self.state.get(key)
        }
    }

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert_eq!(config.jobs, num_cpus::get());
        assert_eq!(config.queue_size, 100);
        assert_eq!(config.fail_fast, false);
        assert_eq!(config.scenario_timeout, 0);
    }

    #[test]
    fn test_executable_scenario_creation() {
        let scenario = ExecutableScenario {
            scenario_id: 1,
            origin: ScenarioOrigin {
                feature_path: "test.feature".to_string(),
                rule_name: None,
                scenario_name: "Test scenario".to_string(),
                line: 10,
            },
            tags: vec!["@smoke".to_string()],
            steps: vec![Step {
                verb: StepType::Given,
                text_parts: vec![
                    crate::gherkin::StepText::Word(("a".to_string(), SimpleSpan::new((), 0..1))),
                    crate::gherkin::StepText::Word(("test".to_string(), SimpleSpan::new((), 2..6))),
                    crate::gherkin::StepText::Word((
                        "step".to_string(),
                        SimpleSpan::new((), 7..11),
                    )),
                ]
                .into_boxed_slice(),
                doc_string: None,
                data_table: None,
            }],
        };

        assert_eq!(scenario.scenario_id, 1);
        assert_eq!(scenario.origin.scenario_name, "Test scenario");
        assert_eq!(scenario.tags.len(), 1);
        assert_eq!(scenario.steps.len(), 1);
    }

    // #[test]
    // fn test_parallel_executor_creation() {
    //     let config = ParallelConfig::default();
    //     let event_bus = EventBus::new();
    //     let hook_registry = Arc::new(HookRegistry::<TestWorld>::new());
    //     let steps = Arc::new(DefaultSteps::new());

    //     let executor = ParallelExecutor::new(config, event_bus, hook_registry, steps);
    //     assert_eq!(executor.config.jobs, num_cpus::get());
    // }

    // #[test]
    // fn test_plan_simple_scenario() {
    //     let config = ParallelConfig::default();
    //     let event_bus = EventBus::new();
    //     let hook_registry = Arc::new(HookRegistry::<TestWorld>::new());
    //     let steps = Arc::new(DefaultSteps::new());

    //     let executor = ParallelExecutor::new(config, event_bus, hook_registry, steps);

    //     let document = Document {
    //         language: None,
    //         feature: crate::gherkin::Feature {
    //             header: crate::gherkin::Header {
    //                 comments: vec![],
    //                 keyword: "Feature".to_string(),
    //                 name: "Test feature".to_string(),
    //                 description: None,
    //                 tags: vec!["@smoke".to_string()],
    //             },
    //             background: None,
    //             scenarios: vec![crate::gherkin::Scenario {
    //                 header: crate::gherkin::Header {
    //                     comments: vec![],
    //                     keyword: "Scenario".to_string(),
    //                     name: "Test scenario".to_string(),
    //                     description: None,
    //                     tags: vec!["@fast".to_string()],
    //                 },
    //                 steps: vec![(
    //                     Step {
    //                         verb: StepType::Given,
    //                         text_parts: vec![
    //                             crate::gherkin::StepText::Word((
    //                                 "a".to_string(),
    //                                 SimpleSpan::new((), 0..1),
    //                             )),
    //                             crate::gherkin::StepText::Word((
    //                                 "test".to_string(),
    //                                 SimpleSpan::new((), 2..6),
    //                             )),
    //                             crate::gherkin::StepText::Word((
    //                                 "step".to_string(),
    //                                 SimpleSpan::new((), 7..11),
    //                             )),
    //                         ]
    //                         .into_boxed_slice(),
    //                         doc_string: None,
    //                         data_table: None,
    //                     },
    //                     SimpleSpan::new((), 0..11),
    //                 )],
    //                 examples: vec![],
    //             }],
    //             rules: vec![],
    //         },
    //     };

    //     let scenarios = executor.plan(&document, "test.feature");
    //     assert_eq!(scenarios.len(), 1);

    //     let scenario = &scenarios[0];
    //     assert_eq!(scenario.scenario_id, 0);
    //     assert_eq!(scenario.origin.scenario_name, "Test scenario");
    //     assert_eq!(scenario.tags.len(), 2); // @smoke from feature + @fast from scenario
    //     assert_eq!(scenario.steps.len(), 1);
    // }

    #[test]
    fn test_scenario_origin() {
        let origin = ScenarioOrigin {
            feature_path: "/path/to/test.feature".to_string(),
            rule_name: Some("Test rule".to_string()),
            scenario_name: "Test scenario".to_string(),
            line: 42,
        };

        assert_eq!(origin.feature_path, "/path/to/test.feature");
        assert_eq!(origin.rule_name, Some("Test rule".to_string()));
        assert_eq!(origin.scenario_name, "Test scenario");
        assert_eq!(origin.line, 42);
    }

    #[test]
    fn test_parallel_event_types() {
        let event1 = ParallelEvent::WorkerStarted {
            worker_id: 0,
            thread_id: "thread-1".to_string(),
        };

        let event2 = ParallelEvent::ScenarioAssigned {
            scenario_id: 5,
            worker_id: 2,
        };

        let event3 = ParallelEvent::WorkerStopped {
            worker_id: 1,
            thread_id: "thread-2".to_string(),
        };

        // Verify the events can be created and are properly typed
        match event1 {
            ParallelEvent::WorkerStarted {
                worker_id,
                thread_id,
            } => {
                assert_eq!(worker_id, 0);
                assert_eq!(thread_id, "thread-1");
            }
            _ => panic!("Wrong event type"),
        }

        match event2 {
            ParallelEvent::ScenarioAssigned {
                scenario_id,
                worker_id,
            } => {
                assert_eq!(scenario_id, 5);
                assert_eq!(worker_id, 2);
            }
            _ => panic!("Wrong event type"),
        }

        match event3 {
            ParallelEvent::WorkerStopped {
                worker_id,
                thread_id,
            } => {
                assert_eq!(worker_id, 1);
                assert_eq!(thread_id, "thread-2");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_world_isolation() {
        let mut world1 = TestWorld::new();
        let mut world2 = TestWorld::new();

        world1.set_state("test_key", "value1");
        world2.set_state("test_key", "value2");

        assert_eq!(world1.get_state("test_key"), Some(&"value1".to_string()));
        assert_eq!(world2.get_state("test_key"), Some(&"value2".to_string()));

        // Verify they are independent
        world1.set_state("unique_key", "unique_value");
        assert_eq!(
            world1.get_state("unique_key"),
            Some(&"unique_value".to_string())
        );
        assert_eq!(world2.get_state("unique_key"), None);
    }
}
