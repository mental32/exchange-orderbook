//! World and Hooks system for cucumber-gherkin
//!
//! This module provides:
//! - World trait for per-scenario state isolation
//! - Hook system (BeforeAll/AfterAll, Before/After) with tag filtering
//! - Failure propagation and skip semantics

use crate::gherkin::TagOperation;

/// World trait - user-defined state that's instantiated per scenario
/// This enables clean state isolation and better concurrency later
pub trait World: Sized + Send + 'static {
    /// Create a new World instance for a scenario
    fn new() -> Self;
}

/// Result of running a step, background, or hook
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// Step passed successfully
    Passed { duration_ms: u64 },
    /// Step failed with an error
    Failed { error: String, duration_ms: u64 },
    /// Step was skipped (due to previous failure)
    Skipped,
    /// Step is undefined (no matching step definition)
    Undefined { step_text: String },
}

impl ExecutionResult {
    pub fn is_failure(&self) -> bool {
        matches!(self, ExecutionResult::Failed { .. })
    }

    pub fn is_passed(&self) -> bool {
        matches!(self, ExecutionResult::Passed { .. })
    }
}

/// Hook function signature for Before/After hooks
pub type HookFn<W> = Box<dyn Fn(&mut W) -> ExecutionResult + Send + Sync>;

/// Hook function signature for BeforeAll/AfterAll hooks
pub type GlobalHookFn = Box<dyn Fn() -> ExecutionResult + Send + Sync>;

/// Hook registration with optional tag filtering
pub struct Hook<W: World> {
    pub name: String,
    pub tag_filter: Option<TagOperation>,
    pub hook_fn: HookFn<W>,
}

/// Global hook registration (BeforeAll/AfterAll)
pub struct GlobalHook {
    pub name: String,
    pub hook_fn: GlobalHookFn,
}

/// Registry for all hooks
pub struct HookRegistry<W: World> {
    pub before_all: Vec<GlobalHook>,
    pub after_all: Vec<GlobalHook>,
    pub before: Vec<Hook<W>>,
    pub after: Vec<Hook<W>>,
}

impl<W: World> HookRegistry<W> {
    pub fn new() -> Self {
        Self {
            before_all: Vec::new(),
            after_all: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
        }
    }

    /// Register a BeforeAll hook (runs once per test run)
    pub fn before_all<F>(&mut self, name: impl Into<String>, hook_fn: F)
    where
        F: Fn() -> ExecutionResult + Send + Sync + 'static,
    {
        let hook_name = name.into();
        dbg!("🎣 Registering BeforeAll hook: {}", &hook_name);
        self.before_all.push(GlobalHook {
            name: hook_name,
            hook_fn: Box::new(hook_fn),
        });
    }

    /// Register an AfterAll hook (runs once per test run)
    pub fn after_all<F>(&mut self, name: impl Into<String>, hook_fn: F)
    where
        F: Fn() -> ExecutionResult + Send + Sync + 'static,
    {
        let hook_name = name.into();
        dbg!("🎣 Registering AfterAll hook: {}", &hook_name);
        self.after_all.push(GlobalHook {
            name: hook_name,
            hook_fn: Box::new(hook_fn),
        });
    }

    /// Register a Before hook (runs per scenario, with optional tag filter)
    pub fn before<F>(
        &mut self,
        name: impl Into<String>,
        tag_filter: Option<TagOperation>,
        hook_fn: F,
    ) where
        F: Fn(&mut W) -> ExecutionResult + Send + Sync + 'static,
    {
        let hook_name = name.into();
        dbg!(
            "🎣 Registering Before hook: {} with tag filter: {:?}",
            &hook_name,
            &tag_filter
        );
        self.before.push(Hook {
            name: hook_name,
            tag_filter,
            hook_fn: Box::new(hook_fn),
        });
    }

    /// Register an After hook (runs per scenario, with optional tag filter)
    pub fn after<F>(
        &mut self,
        name: impl Into<String>,
        tag_filter: Option<TagOperation>,
        hook_fn: F,
    ) where
        F: Fn(&mut W) -> ExecutionResult + Send + Sync + 'static,
    {
        let hook_name = name.into();
        dbg!(
            "🎣 Registering After hook: {} with tag filter: {:?}",
            &hook_name,
            &tag_filter
        );
        self.after.push(Hook {
            name: hook_name,
            tag_filter,
            hook_fn: Box::new(hook_fn),
        });
    }

    /// Get Before hooks that should run for the given tags
    pub fn get_before_hooks(&self, tags: &[String]) -> Vec<&Hook<W>> {
        self.before
            .iter()
            .filter(|hook| {
                if let Some(ref filter) = hook.tag_filter {
                    let should_run = filter.evaluate(tags);
                    dbg!(
                        "🏷️ Before hook '{}' tag filter evaluation: {:?} against tags {:?} = {}",
                        &hook.name,
                        filter,
                        tags,
                        should_run
                    );
                    should_run
                } else {
                    dbg!(
                        "✅ Before hook '{}' has no tag filter, will run",
                        &hook.name
                    );
                    true
                }
            })
            .collect()
    }

    /// Get After hooks that should run for the given tags (always run, even after failures)
    pub fn get_after_hooks(&self, tags: &[String]) -> Vec<&Hook<W>> {
        self.after
            .iter()
            .filter(|hook| {
                if let Some(ref filter) = hook.tag_filter {
                    let should_run = filter.evaluate(tags);
                    dbg!(
                        "🏷️ After hook '{}' tag filter evaluation: {:?} against tags {:?} = {}",
                        &hook.name,
                        filter,
                        tags,
                        should_run
                    );
                    should_run
                } else {
                    dbg!("✅ After hook '{}' has no tag filter, will run", &hook.name);
                    true
                }
            })
            .collect()
    }

    /// Run BeforeAll hooks
    pub fn run_before_all(&self) -> Vec<ExecutionResult> {
        dbg!("🎣 Running {} BeforeAll hooks", self.before_all.len());
        self.before_all
            .iter()
            .map(|hook| {
                dbg!("▶️ Executing BeforeAll hook: {}", &hook.name);
                (hook.hook_fn)()
            })
            .collect()
    }

    /// Run AfterAll hooks
    pub fn run_after_all(&self) -> Vec<ExecutionResult> {
        dbg!("🎣 Running {} AfterAll hooks", self.after_all.len());
        self.after_all
            .iter()
            .map(|hook| {
                dbg!("▶️ Executing AfterAll hook: {}", &hook.name);
                (hook.hook_fn)()
            })
            .collect()
    }
}

impl<W: World> Default for HookRegistry<W> {
    fn default() -> Self {
        Self::new()
    }
}

/// Default World implementation for simple use cases
#[derive(Debug, Clone)]
pub struct DefaultWorld {
    pub state: std::collections::HashMap<String, String>,
}

impl World for DefaultWorld {
    fn new() -> Self {
        Self {
            state: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gherkin::TagOperation;

    #[test]
    fn test_world_creation() {
        let world = DefaultWorld::new();
        assert!(world.state.is_empty());
    }

    #[test]
    fn test_hook_registry_creation() {
        let registry: HookRegistry<DefaultWorld> = HookRegistry::new();
        assert_eq!(registry.before_all.len(), 0);
        assert_eq!(registry.after_all.len(), 0);
        assert_eq!(registry.before.len(), 0);
        assert_eq!(registry.after.len(), 0);
    }

    #[test]
    fn test_before_all_hook_registration() {
        let mut registry: HookRegistry<DefaultWorld> = HookRegistry::new();

        registry.before_all("test_hook", || ExecutionResult::Passed { duration_ms: 1 });

        assert_eq!(registry.before_all.len(), 1);
        assert_eq!(registry.before_all[0].name, "test_hook");
    }

    #[test]
    fn test_after_all_hook_registration() {
        let mut registry: HookRegistry<DefaultWorld> = HookRegistry::new();

        registry.after_all("cleanup_hook", || ExecutionResult::Passed {
            duration_ms: 2,
        });

        assert_eq!(registry.after_all.len(), 1);
        assert_eq!(registry.after_all[0].name, "cleanup_hook");
    }

    #[test]
    fn test_before_hook_registration_no_filter() {
        let mut registry: HookRegistry<DefaultWorld> = HookRegistry::new();

        registry.before("setup_hook", None, |_world| ExecutionResult::Passed {
            duration_ms: 3,
        });

        assert_eq!(registry.before.len(), 1);
        assert_eq!(registry.before[0].name, "setup_hook");
        assert!(registry.before[0].tag_filter.is_none());
    }

    #[test]
    fn test_before_hook_registration_with_tag_filter() {
        let mut registry: HookRegistry<DefaultWorld> = HookRegistry::new();
        let tag_expr = TagOperation::Tag("smoke".to_owned());

        registry.before("smoke_setup", Some(tag_expr.clone()), |_world| {
            ExecutionResult::Passed { duration_ms: 4 }
        });

        assert_eq!(registry.before.len(), 1);
        assert_eq!(registry.before[0].name, "smoke_setup");
        assert_eq!(registry.before[0].tag_filter, Some(tag_expr));
    }

    #[test]
    fn test_get_before_hooks_no_filter() {
        let mut registry: HookRegistry<DefaultWorld> = HookRegistry::new();

        registry.before("always_run", None, |_world| ExecutionResult::Passed {
            duration_ms: 5,
        });

        let tags = vec!["smoke".to_owned(), "fast".to_owned()];
        let hooks = registry.get_before_hooks(&tags);

        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].name, "always_run");
    }

    #[test]
    fn test_get_before_hooks_with_matching_tag_filter() {
        let mut registry: HookRegistry<DefaultWorld> = HookRegistry::new();
        let tag_expr = TagOperation::Tag("smoke".to_owned());

        registry.before("smoke_setup", Some(tag_expr), |_world| {
            ExecutionResult::Passed { duration_ms: 6 }
        });

        let tags = vec!["smoke".to_owned(), "fast".to_owned()];
        let hooks = registry.get_before_hooks(&tags);

        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].name, "smoke_setup");
    }

    #[test]
    fn test_get_before_hooks_with_non_matching_tag_filter() {
        let mut registry: HookRegistry<DefaultWorld> = HookRegistry::new();
        let tag_expr = TagOperation::Tag("integration".to_owned());

        registry.before("integration_setup", Some(tag_expr), |_world| {
            ExecutionResult::Passed { duration_ms: 7 }
        });

        let tags = vec!["smoke".to_owned(), "fast".to_owned()];
        let hooks = registry.get_before_hooks(&tags);

        assert_eq!(hooks.len(), 0);
    }

    #[test]
    fn test_run_before_all_hooks() {
        let mut registry: HookRegistry<DefaultWorld> = HookRegistry::new();

        registry.before_all("first", || ExecutionResult::Passed { duration_ms: 10 });
        registry.before_all("second", || ExecutionResult::Failed {
            error: "test failure".to_owned(),
            duration_ms: 5,
        });

        let results = registry.run_before_all();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0], ExecutionResult::Passed { duration_ms: 10 });
        assert_eq!(
            results[1],
            ExecutionResult::Failed {
                error: "test failure".to_owned(),
                duration_ms: 5
            }
        );
    }

    #[test]
    fn test_execution_result_is_failure() {
        assert!(
            ExecutionResult::Failed {
                error: "error".to_owned(),
                duration_ms: 1
            }
            .is_failure()
        );
        assert!(!ExecutionResult::Passed { duration_ms: 1 }.is_failure());
        assert!(!ExecutionResult::Skipped.is_failure());
        assert!(
            !ExecutionResult::Undefined {
                step_text: "undefined".to_owned()
            }
            .is_failure()
        );
    }

    #[test]
    fn test_execution_result_is_passed() {
        assert!(
            !ExecutionResult::Failed {
                error: "error".to_owned(),
                duration_ms: 1
            }
            .is_passed()
        );
        assert!(ExecutionResult::Passed { duration_ms: 1 }.is_passed());
        assert!(!ExecutionResult::Skipped.is_passed());
        assert!(
            !ExecutionResult::Undefined {
                step_text: "undefined".to_owned()
            }
            .is_passed()
        );
    }
}
