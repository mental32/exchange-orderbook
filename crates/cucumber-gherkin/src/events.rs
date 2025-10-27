/// Event Bus system for cucumber-gherkin
///
/// Provides publish-subscribe event system for test execution lifecycle,
/// enabling reporters, logging, and real-time monitoring of test runs.
///
/// Events follow the Oracle guidance for comprehensive observability:
/// - TestRunStarted/TestRunFinished for overall execution
/// - FeatureStarted/FeatureFinished for feature-level reporting
/// - ScenarioStarted/ScenarioFinished for scenario-level tracking
/// - StepStarted/StepFinished for step-level observability
/// - Attachment events for screenshots, logs, etc.
use crate::results::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::SystemTime;

/// Event types in the test execution lifecycle
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TestEvent {
    /// Test run started
    TestRunStarted {
        start_time: SystemTime,
        build_metadata: HashMap<String, String>,
    },

    /// Test run finished
    TestRunFinished {
        end_time: SystemTime,
        run_result: RunResult,
    },

    /// Feature started
    FeatureStarted {
        feature_id: String,
        name: String,
        file_path: String,
        tags: Vec<String>,
        start_time: SystemTime,
    },

    /// Feature finished
    FeatureFinished {
        feature_id: String,
        feature_result: FeatureResult,
    },

    /// Scenario started
    ScenarioStarted {
        scenario_id: TestId,
        name: String,
        tags: Vec<String>,
        outline_info: Option<OutlineInfo>,
        start_time: SystemTime,
    },

    /// Scenario finished
    ScenarioFinished {
        scenario_id: TestId,
        scenario_result: ScenarioResult,
    },

    /// Step started
    StepStarted {
        step_id: String,
        scenario_id: TestId,
        name: String,
        keyword: String,
        location: Option<StepLocation>,
        start_time: SystemTime,
    },

    /// Step finished
    StepFinished {
        step_id: String,
        scenario_id: TestId,
        step_result: StepResult,
    },

    /// Attachment added (screenshots, logs, etc.)
    Attachment {
        step_id: Option<String>,
        scenario_id: TestId,
        attachment: Attachment,
    },

    /// Hook started (Before/After/BeforeAll/AfterAll)
    HookStarted {
        hook_name: String,
        hook_type: HookType,
        scenario_id: Option<TestId>,
        start_time: SystemTime,
    },

    /// Hook finished
    HookFinished {
        hook_name: String,
        hook_type: HookType,
        scenario_id: Option<TestId>,
        duration: std::time::Duration,
        status: HookStatus,
        error: Option<String>,
    },
}

/// Hook types for hook events
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HookType {
    BeforeAll,
    AfterAll,
    Before,
    After,
}

/// Hook execution status
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HookStatus {
    Passed,
    Failed,
    Skipped,
}

/// Event listener trait for handling test events
pub trait EventListener: Send + Sync {
    /// Handle a test event
    fn on_event(&mut self, event: &TestEvent);

    /// Get listener name for debugging
    fn name(&self) -> &str {
        "EventListener"
    }
}

/// Event bus for publishing and subscribing to test events
pub struct EventBus {
    listeners: Arc<RwLock<Vec<Arc<Mutex<dyn EventListener>>>>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Subscribe a listener to events
    pub fn subscribe<L: EventListener + 'static>(&self, listener: L) {
        let mut listeners = self.listeners.write().unwrap();
        listeners.push(Arc::new(Mutex::new(listener)));
        dbg!("Subscribed listener to event bus");
    }

    /// Publish an event to all listeners
    pub fn publish(&self, event: TestEvent) {
        dbg!("Publishing event: {:?}", &event);
        let listeners = self.listeners.read().unwrap();

        for listener in listeners.iter() {
            if let Ok(mut listener) = listener.try_lock() {
                listener.on_event(&event);
            } else {
                eprintln!("Warning: Could not acquire lock on event listener");
            }
        }
    }

    /// Get number of subscribed listeners
    pub fn listener_count(&self) -> usize {
        self.listeners.read().unwrap().len()
    }

    /// Clear all listeners (useful for testing)
    pub fn clear_listeners(&self) {
        let mut listeners = self.listeners.write().unwrap();
        listeners.clear();
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            listeners: Arc::clone(&self.listeners),
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple logging event listener for debugging
pub struct LoggingListener {
    pub name: String,
}

impl LoggingListener {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl EventListener for LoggingListener {
    fn on_event(&mut self, event: &TestEvent) {
        match event {
            TestEvent::TestRunStarted { .. } => {
                println!("📊 {} - Test run started", self.name);
            }
            TestEvent::TestRunFinished { run_result, .. } => {
                println!(
                    "📊 {} - Test run finished: {} scenarios, {} passed",
                    self.name, run_result.total_scenarios, run_result.passed_scenarios
                );
            }
            TestEvent::FeatureStarted { name, .. } => {
                println!("📋 {} - Feature started: {}", self.name, name);
            }
            TestEvent::FeatureFinished { feature_result, .. } => {
                let (passed, failed, _total) = feature_result.scenario_counts();
                println!(
                    "📋 {} - Feature finished: {} ({} passed, {} failed)",
                    self.name, feature_result.name, passed, failed
                );
            }
            TestEvent::ScenarioStarted { name, .. } => {
                println!("🎬 {} - Scenario started: {}", self.name, name);
            }
            TestEvent::ScenarioFinished {
                scenario_result, ..
            } => {
                println!(
                    "🎬 {} - Scenario finished: {} ({})",
                    self.name,
                    scenario_result.name,
                    if scenario_result.status.is_passed() {
                        "✅"
                    } else {
                        "❌"
                    }
                );
            }
            TestEvent::StepStarted { name, keyword, .. } => {
                println!("🚶 {} - Step started: {} {}", self.name, keyword, name);
            }
            TestEvent::StepFinished { step_result, .. } => {
                println!(
                    "🚶 {} - Step finished: {} {} ({})",
                    self.name,
                    step_result.keyword,
                    step_result.name,
                    if step_result.status.is_passed() {
                        "✅"
                    } else {
                        "❌"
                    }
                );
            }
            TestEvent::Attachment { attachment, .. } => {
                println!(
                    "📎 {} - Attachment: {} ({})",
                    self.name,
                    attachment.name.as_ref().unwrap_or(&"unnamed".to_owned()),
                    attachment.media_type
                );
            }
            TestEvent::HookStarted {
                hook_name,
                hook_type,
                ..
            } => {
                println!(
                    "🎣 {} - Hook started: {} ({:?})",
                    self.name, hook_name, hook_type
                );
            }
            TestEvent::HookFinished {
                hook_name,
                hook_type,
                status,
                ..
            } => {
                println!(
                    "🎣 {} - Hook finished: {} ({:?}) - {:?}",
                    self.name, hook_name, hook_type, status
                );
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Event collector for testing - collects all events
pub struct EventCollector {
    pub events: Arc<Mutex<Vec<TestEvent>>>,
    pub name: String,
}

impl EventCollector {
    pub fn new(name: String) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            name,
        }
    }

    /// Get collected events
    pub fn get_events(&self) -> Vec<TestEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Clear collected events
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Count events of a specific type
    pub fn count_events<F>(&self, filter: F) -> usize
    where
        F: Fn(&TestEvent) -> bool,
    {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| filter(e))
            .count()
    }
}

impl EventListener for EventCollector {
    fn on_event(&mut self, event: &TestEvent) {
        self.events.lock().unwrap().push(event.clone());
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new();
        assert_eq!(bus.listener_count(), 0);
    }

    #[test]
    fn test_event_bus_subscribe() {
        let bus = EventBus::new();
        let collector = EventCollector::new("test_collector".to_owned());

        bus.subscribe(collector);
        assert_eq!(bus.listener_count(), 1);
    }

    #[test]
    fn test_event_bus_publish() {
        let bus = EventBus::new();
        let collector = EventCollector::new("test_collector".to_owned());
        let collector_events = Arc::clone(&collector.events);

        bus.subscribe(collector);

        let event = TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: HashMap::new(),
        };

        bus.publish(event);

        assert_eq!(collector_events.lock().unwrap().len(), 1);

        // Check event type
        let events = collector_events.lock().unwrap();
        matches!(events[0], TestEvent::TestRunStarted { .. });
    }

    #[test]
    fn test_multiple_listeners() {
        let bus = EventBus::new();
        let collector1 = EventCollector::new("collector1".to_owned());
        let collector2 = EventCollector::new("collector2".to_owned());

        let events1 = Arc::clone(&collector1.events);
        let events2 = Arc::clone(&collector2.events);

        bus.subscribe(collector1);
        bus.subscribe(collector2);

        let event = TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: HashMap::new(),
        };

        bus.publish(event);

        // Both collectors should receive the event
        assert_eq!(events1.lock().unwrap().len(), 1);
        assert_eq!(events2.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_event_collector_filtering() {
        let mut collector = EventCollector::new("test".to_owned());

        // Simulate receiving different events
        collector.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: HashMap::new(),
        });

        collector.on_event(&TestEvent::FeatureStarted {
            feature_id: "feature1".to_owned(),
            name: "Test Feature".to_owned(),
            file_path: "test.feature".to_owned(),
            tags: vec![],
            start_time: SystemTime::now(),
        });

        collector.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: HashMap::new(),
        });

        // Count TestRunStarted events
        let test_run_count =
            collector.count_events(|e| matches!(e, TestEvent::TestRunStarted { .. }));
        assert_eq!(test_run_count, 2);

        // Count FeatureStarted events
        let feature_count =
            collector.count_events(|e| matches!(e, TestEvent::FeatureStarted { .. }));
        assert_eq!(feature_count, 1);

        assert_eq!(collector.get_events().len(), 3);
    }

    #[test]
    fn test_logging_listener() {
        let mut listener = LoggingListener::new("TestLogger".to_owned());

        // This should not panic - just verify the interface works
        listener.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: HashMap::new(),
        });

        assert_eq!(listener.name(), "TestLogger");
    }

    #[test]
    fn test_event_bus_clear_listeners() {
        let bus = EventBus::new();
        bus.subscribe(EventCollector::new("test".to_owned()));

        assert_eq!(bus.listener_count(), 1);

        bus.clear_listeners();
        assert_eq!(bus.listener_count(), 0);
    }
}
