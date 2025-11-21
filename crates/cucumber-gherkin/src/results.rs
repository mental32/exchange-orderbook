use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;

/// Unique identifier for test execution components
/// Uses content-derived ID for deterministic output and reruns
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TestId {
    pub feature_path: String,
    pub scenario_line: usize,
    pub scenario_name: String,
    /// For Scenario Outlines: row index in Examples table
    pub row_index: Option<usize>,
}

impl TestId {
    pub fn new(feature_path: String, scenario_line: usize, scenario_name: String) -> Self {
        Self {
            feature_path,
            scenario_line,
            scenario_name,
            row_index: None,
        }
    }

    pub fn with_row_index(mut self, row_index: usize) -> Self {
        self.row_index = Some(row_index);
        self
    }

    /// Generate stable string ID for deterministic output
    pub fn as_string(&self) -> String {
        match self.row_index {
            Some(idx) => format!(
                "{}:{}:{}:{}",
                self.feature_path, self.scenario_line, self.scenario_name, idx
            ),
            None => format!(
                "{}:{}:{}",
                self.feature_path, self.scenario_line, self.scenario_name
            ),
        }
    }
}

/// Step execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Step executed successfully
    Passed,
    /// Step failed with error
    Failed,
    /// Step was skipped due to previous failure
    Skipped,
    /// Step definition exists but not implemented
    Pending,
    /// No step definition found
    Undefined,
    /// Multiple step definitions match (ambiguous)
    Ambiguous,
}

impl StepStatus {
    pub fn is_passed(&self) -> bool {
        matches!(self, StepStatus::Passed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, StepStatus::Failed)
    }

    pub fn is_error(&self) -> bool {
        matches!(
            self,
            StepStatus::Failed | StepStatus::Undefined | StepStatus::Ambiguous
        )
    }
}

/// Error information for failed steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepError {
    pub message: String,
    pub error_type: String,
    pub stack_trace: Option<String>,
    pub location: Option<StepLocation>,
}

/// Step location in source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepLocation {
    pub file_path: String,
    pub line_number: usize,
    pub column: Option<usize>,
}

/// Attachment (screenshots, logs, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub media_type: String,
    pub name: Option<String>,
    pub body: AttachmentBody,
    pub timestamp: SystemTime,
}

/// Attachment body content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentBody {
    /// Text content
    Text(String),
    /// Binary content (base64 encoded when serialized)
    Binary(Vec<u8>),
}

/// Result of a single step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub id: String,
    pub name: String,
    pub keyword: String, // Given, When, Then, And, But, *
    pub location: Option<StepLocation>,
    pub status: StepStatus,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub duration: Duration,
    pub error: Option<StepError>,
    pub attachments: Vec<Attachment>,
    pub attempt: usize, // For retries: 1, 2, 3...
}

impl StepResult {
    pub fn new(id: String, name: String, keyword: String) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name,
            keyword,
            location: None,
            status: StepStatus::Undefined,
            start_time: now,
            end_time: now,
            duration: Duration::from_millis(0),
            error: None,
            attachments: Vec::new(),
            attempt: 1,
        }
    }

    pub fn with_status(mut self, status: StepStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_duration(mut self, start: SystemTime, end: SystemTime) -> Self {
        self.start_time = start;
        self.end_time = end;
        self.duration = end.duration_since(start).unwrap_or_default();
        self
    }

    pub fn with_error(mut self, error: StepError) -> Self {
        self.error = Some(error);
        if self.status == StepStatus::Undefined {
            self.status = StepStatus::Failed;
        }
        self
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration.as_millis() as u64
    }
}

/// Scenario execution status (aggregated from steps)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioStatus {
    Passed,
    Failed,
    Skipped,
    Pending,
    Undefined,
    Ambiguous,
}

impl ScenarioStatus {
    /// Compute scenario status from step statuses
    pub fn from_steps(step_results: &[StepResult]) -> Self {
        if step_results.is_empty() {
            return ScenarioStatus::Undefined;
        }

        // Check for any undefined or ambiguous steps
        if step_results
            .iter()
            .any(|s| s.status == StepStatus::Undefined)
        {
            return ScenarioStatus::Undefined;
        }
        if step_results
            .iter()
            .any(|s| s.status == StepStatus::Ambiguous)
        {
            return ScenarioStatus::Ambiguous;
        }

        // Check for failures
        if step_results.iter().any(|s| s.status == StepStatus::Failed) {
            return ScenarioStatus::Failed;
        }

        // Check for pending
        if step_results.iter().any(|s| s.status == StepStatus::Pending) {
            return ScenarioStatus::Pending;
        }

        // All steps passed or skipped
        if step_results
            .iter()
            .all(|s| s.status.is_passed() || s.status == StepStatus::Skipped)
        {
            ScenarioStatus::Passed
        } else {
            ScenarioStatus::Failed
        }
    }

    pub fn is_passed(&self) -> bool {
        matches!(self, ScenarioStatus::Passed)
    }

    pub fn is_failed(&self) -> bool {
        !self.is_passed()
    }
}

/// Scenario Outline parameter values for a specific example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineInfo {
    pub row_index: usize,
    pub parameter_values: HashMap<String, String>,
}

/// Result of a scenario execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub id: TestId,
    pub name: String,
    pub tags: Vec<String>,
    pub outline_info: Option<OutlineInfo>,
    pub steps: Vec<StepResult>,
    pub status: ScenarioStatus,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub duration: Duration,
    pub retry_count: usize,
    pub attachments: Vec<Attachment>, // Scenario-level attachments
}

impl ScenarioResult {
    pub fn new(id: TestId, name: String, tags: Vec<String>) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name,
            tags,
            outline_info: None,
            steps: Vec::new(),
            status: ScenarioStatus::Undefined,
            start_time: now,
            end_time: now,
            duration: Duration::from_millis(0),
            retry_count: 0,
            attachments: Vec::new(),
        }
    }

    pub fn with_steps(mut self, steps: Vec<StepResult>) -> Self {
        self.status = ScenarioStatus::from_steps(&steps);
        self.steps = steps;
        self
    }

    pub fn with_duration(mut self, start: SystemTime, end: SystemTime) -> Self {
        self.start_time = start;
        self.end_time = end;
        self.duration = end.duration_since(start).unwrap_or_default();
        self
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration.as_millis() as u64
    }

    /// Get the first failing step, if any
    pub fn first_failure(&self) -> Option<&StepResult> {
        self.steps.iter().find(|s| s.status.is_failed())
    }
}

/// Feature execution status (aggregated from scenarios)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureStatus {
    Passed,
    Failed,
    Skipped,
    Undefined,
}

impl FeatureStatus {
    pub fn from_scenarios(scenario_results: &[ScenarioResult]) -> Self {
        if scenario_results.is_empty() {
            return FeatureStatus::Undefined;
        }

        if scenario_results.iter().any(|s| s.status.is_failed()) {
            FeatureStatus::Failed
        } else if scenario_results.iter().all(|s| s.status.is_passed()) {
            FeatureStatus::Passed
        } else {
            FeatureStatus::Undefined
        }
    }

    pub fn is_passed(&self) -> bool {
        matches!(self, FeatureStatus::Passed)
    }
}

/// Result of a feature execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureResult {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub file_path: String,
    pub scenarios: Vec<ScenarioResult>,
    pub status: FeatureStatus,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub duration: Duration,
}

impl FeatureResult {
    pub fn new(id: String, name: String, file_path: String, tags: Vec<String>) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name,
            tags,
            file_path,
            scenarios: Vec::new(),
            status: FeatureStatus::Undefined,
            start_time: now,
            end_time: now,
            duration: Duration::from_millis(0),
        }
    }

    pub fn with_scenarios(mut self, scenarios: Vec<ScenarioResult>) -> Self {
        self.status = FeatureStatus::from_scenarios(&scenarios);
        self.scenarios = scenarios;
        self
    }

    pub fn with_duration(mut self, start: SystemTime, end: SystemTime) -> Self {
        self.start_time = start;
        self.end_time = end;
        self.duration = end.duration_since(start).unwrap_or_default();
        self
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration.as_millis() as u64
    }

    /// Count scenarios by status
    pub fn scenario_counts(&self) -> (usize, usize, usize) {
        let passed = self
            .scenarios
            .iter()
            .filter(|s| s.status.is_passed())
            .count();
        let failed = self
            .scenarios
            .iter()
            .filter(|s| s.status.is_failed())
            .count();
        let total = self.scenarios.len();
        (passed, failed, total)
    }
}

/// Overall test run result and summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub features: Vec<FeatureResult>,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub duration: Duration,
    pub build_metadata: HashMap<String, String>,
    pub total_scenarios: usize,
    pub passed_scenarios: usize,
    pub failed_scenarios: usize,
    pub flaky_scenarios: usize, // Passed after retry
    pub total_steps: usize,
    pub passed_steps: usize,
    pub failed_steps: usize,
    pub skipped_steps: usize,
    pub undefined_steps: usize,
    pub pending_steps: usize,
}

impl RunResult {
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            features: Vec::new(),
            start_time: now,
            end_time: now,
            duration: Duration::from_millis(0),
            build_metadata: HashMap::new(),
            total_scenarios: 0,
            passed_scenarios: 0,
            failed_scenarios: 0,
            flaky_scenarios: 0,
            total_steps: 0,
            passed_steps: 0,
            failed_steps: 0,
            skipped_steps: 0,
            undefined_steps: 0,
            pending_steps: 0,
        }
    }

    pub fn with_features(mut self, features: Vec<FeatureResult>) -> Self {
        self.features = features;
        self.calculate_totals();
        self
    }

    pub fn with_duration(mut self, start: SystemTime, end: SystemTime) -> Self {
        self.start_time = start;
        self.end_time = end;
        self.duration = end.duration_since(start).unwrap_or_default();
        self
    }

    /// Calculate aggregate statistics from features
    fn calculate_totals(&mut self) {
        self.total_scenarios = 0;
        self.passed_scenarios = 0;
        self.failed_scenarios = 0;
        self.flaky_scenarios = 0;
        self.total_steps = 0;
        self.passed_steps = 0;
        self.failed_steps = 0;
        self.skipped_steps = 0;
        self.undefined_steps = 0;
        self.pending_steps = 0;

        for feature in &self.features {
            for scenario in &feature.scenarios {
                self.total_scenarios += 1;

                if scenario.status.is_passed() {
                    self.passed_scenarios += 1;
                    // Check if it was flaky (passed after retry)
                    if scenario.retry_count > 0 {
                        self.flaky_scenarios += 1;
                    }
                } else {
                    self.failed_scenarios += 1;
                }

                for step in &scenario.steps {
                    self.total_steps += 1;
                    match step.status {
                        StepStatus::Passed => self.passed_steps += 1,
                        StepStatus::Failed => self.failed_steps += 1,
                        StepStatus::Skipped => self.skipped_steps += 1,
                        StepStatus::Pending => self.pending_steps += 1,
                        StepStatus::Undefined => self.undefined_steps += 1,
                        StepStatus::Ambiguous => self.failed_steps += 1, // Count as failure
                    }
                }
            }
        }
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration.as_millis() as u64
    }

    /// Check if the test run was successful (all scenarios passed)
    pub fn is_success(&self) -> bool {
        self.failed_scenarios == 0 && self.undefined_steps == 0
    }

    /// Get pass percentage
    pub fn pass_percentage(&self) -> f64 {
        if self.total_scenarios == 0 {
            0.0
        } else {
            (self.passed_scenarios as f64) / (self.total_scenarios as f64) * 100.0
        }
    }
}

impl Default for RunResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_id_string_generation() {
        let id = TestId::new(
            "features/test.feature".to_owned(),
            10,
            "Basic scenario".to_owned(),
        );
        assert_eq!(id.as_string(), "features/test.feature:10:Basic scenario");

        let outline_id = id.with_row_index(2);
        assert_eq!(
            outline_id.as_string(),
            "features/test.feature:10:Basic scenario:2"
        );
    }

    #[test]
    fn test_step_status_checks() {
        assert!(StepStatus::Passed.is_passed());
        assert!(!StepStatus::Failed.is_passed());

        assert!(StepStatus::Failed.is_failed());
        assert!(!StepStatus::Passed.is_failed());

        assert!(StepStatus::Failed.is_error());
        assert!(StepStatus::Undefined.is_error());
        assert!(StepStatus::Ambiguous.is_error());
        assert!(!StepStatus::Passed.is_error());
    }

    #[test]
    fn test_scenario_status_from_steps() {
        // All passed
        let step_results = vec![
            StepResult::new("1".to_owned(), "step 1".to_owned(), "Given".to_owned())
                .with_status(StepStatus::Passed),
            StepResult::new("2".to_owned(), "step 2".to_owned(), "When".to_owned())
                .with_status(StepStatus::Passed),
        ];
        assert_eq!(
            ScenarioStatus::from_steps(&step_results),
            ScenarioStatus::Passed
        );

        // One failed
        let step_results = vec![
            StepResult::new("1".to_owned(), "step 1".to_owned(), "Given".to_owned())
                .with_status(StepStatus::Passed),
            StepResult::new("2".to_owned(), "step 2".to_owned(), "When".to_owned())
                .with_status(StepStatus::Failed),
            StepResult::new("3".to_owned(), "step 3".to_owned(), "Then".to_owned())
                .with_status(StepStatus::Skipped),
        ];
        assert_eq!(
            ScenarioStatus::from_steps(&step_results),
            ScenarioStatus::Failed
        );

        // One undefined
        let step_results = vec![
            StepResult::new("1".to_owned(), "step 1".to_owned(), "Given".to_owned())
                .with_status(StepStatus::Undefined),
        ];
        assert_eq!(
            ScenarioStatus::from_steps(&step_results),
            ScenarioStatus::Undefined
        );

        // Empty
        assert_eq!(ScenarioStatus::from_steps(&[]), ScenarioStatus::Undefined);
    }

    #[test]
    fn test_feature_status_from_scenarios() {
        let scenarios = vec![
            ScenarioResult::new(
                TestId::new("test.feature".to_owned(), 1, "Scenario 1".to_owned()),
                "Scenario 1".to_owned(),
                vec![],
            )
            .with_steps(vec![
                StepResult::new("1".to_owned(), "step".to_owned(), "Given".to_owned())
                    .with_status(StepStatus::Passed),
            ]),
        ];

        assert_eq!(
            FeatureStatus::from_scenarios(&scenarios),
            FeatureStatus::Passed
        );

        // Empty
        assert_eq!(FeatureStatus::from_scenarios(&[]), FeatureStatus::Undefined);
    }

    #[test]
    fn test_run_result_calculations() {
        let features = vec![
            FeatureResult::new(
                "feature1".to_owned(),
                "Feature 1".to_owned(),
                "test1.feature".to_owned(),
                vec![],
            )
            .with_scenarios(vec![
                ScenarioResult::new(
                    TestId::new("test1.feature".to_owned(), 1, "Scenario 1".to_owned()),
                    "Scenario 1".to_owned(),
                    vec![],
                )
                .with_steps(vec![
                    StepResult::new("1".to_owned(), "step".to_owned(), "Given".to_owned())
                        .with_status(StepStatus::Passed),
                    StepResult::new("2".to_owned(), "step".to_owned(), "When".to_owned())
                        .with_status(StepStatus::Failed),
                ]),
            ]),
        ];

        let run_result = RunResult::new().with_features(features);

        assert_eq!(run_result.total_scenarios, 1);
        assert_eq!(run_result.failed_scenarios, 1);
        assert_eq!(run_result.passed_scenarios, 0);
        assert_eq!(run_result.total_steps, 2);
        assert_eq!(run_result.passed_steps, 1);
        assert_eq!(run_result.failed_steps, 1);
        assert!(!run_result.is_success());
        assert_eq!(run_result.pass_percentage(), 0.0);
    }
}
