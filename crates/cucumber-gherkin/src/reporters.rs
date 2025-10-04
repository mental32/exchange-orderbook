/// Reporter implementations for cucumber-gherkin
///
/// Provides various output formats based on Oracle guidance:
/// - Pretty: Human-readable nested output with colors and formatting
/// - Progress: Dots/characters with failure summary  
/// - JUnit XML: CI-friendly XML format
/// - Cucumber Messages NDJSON: Rich HTML reporting ecosystem integration
///
/// All reporters implement EventListener to receive real-time test events
use crate::events::EventListener;
/// Reporter implementations for cucumber-gherkin
///
/// Provides various output formats based on Oracle guidance:
/// - Pretty: Human-readable nested output with colors and formatting
/// - Progress: Dots/characters with failure summary  
/// - JUnit XML: CI-friendly XML format
/// - Cucumber Messages NDJSON: Rich HTML reporting ecosystem integration
///
/// All reporters implement EventListener to receive real-time test events
use crate::events::HookStatus;
/// Reporter implementations for cucumber-gherkin
///
/// Provides various output formats based on Oracle guidance:
/// - Pretty: Human-readable nested output with colors and formatting
/// - Progress: Dots/characters with failure summary  
/// - JUnit XML: CI-friendly XML format
/// - Cucumber Messages NDJSON: Rich HTML reporting ecosystem integration
///
/// All reporters implement EventListener to receive real-time test events
use crate::events::HookType;
/// Reporter implementations for cucumber-gherkin
///
/// Provides various output formats based on Oracle guidance:
/// - Pretty: Human-readable nested output with colors and formatting
/// - Progress: Dots/characters with failure summary  
/// - JUnit XML: CI-friendly XML format
/// - Cucumber Messages NDJSON: Rich HTML reporting ecosystem integration
///
/// All reporters implement EventListener to receive real-time test events
use crate::events::TestEvent;
use crate::results::FeatureResult;
use crate::results::FeatureStatus;
use crate::results::OutlineInfo;
use crate::results::RunResult;
use crate::results::ScenarioResult;
use crate::results::ScenarioStatus;
use crate::results::StepStatus;
use crate::results::TestId;
use std::io::Write;
use std::io::{self};
use yansi::Color;
use yansi::Paint;

/// Pretty console reporter with human-readable output
///
/// Features:
/// - Nested indentation for features/scenarios/steps
/// - Color coding for status (green=pass, red=fail, yellow=pending)
/// - Tags and metadata display
/// - DocStrings and DataTables formatting
/// - Real-time output as tests execute
pub struct PrettyReporter<W: Write> {
    writer: W,
    current_feature: Option<String>,
    current_scenario: Option<TestId>,
    scenario_step_count: usize,
    failed_scenarios: Vec<String>,
    show_colors: bool,
    verbose: bool,
}

impl<W: Write> PrettyReporter<W> {
    /// Create new Pretty reporter writing to the given writer
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            current_feature: None,
            current_scenario: None,
            scenario_step_count: 0,
            failed_scenarios: Vec::new(),
            show_colors: true,
            verbose: false,
        }
    }

    /// Create Pretty reporter with custom settings
    pub fn with_options(writer: W, show_colors: bool, verbose: bool) -> Self {
        Self {
            writer,
            current_feature: None,
            current_scenario: None,
            scenario_step_count: 0,
            failed_scenarios: Vec::new(),
            show_colors,
            verbose,
        }
    }

    /// Format text with color if colors are enabled
    fn colorize(&self, text: &str, color: Color) -> String {
        if self.show_colors {
            Paint::new(text).fg(color).to_string()
        } else {
            text.to_string()
        }
    }

    /// Format text with bold if colors are enabled
    fn format_bold(&self, text: &str) -> String {
        if self.show_colors {
            Paint::new(text).bold().to_string()
        } else {
            text.to_string()
        }
    }

    /// Format text with underline if colors are enabled
    fn format_underline(&self, text: &str) -> String {
        if self.show_colors {
            Paint::new(text).underline().to_string()
        } else {
            text.to_string()
        }
    }

    /// Write formatted line with proper indentation
    fn write_line(&mut self, indent: usize, text: &str) -> io::Result<()> {
        let prefix = "  ".repeat(indent);
        writeln!(self.writer, "{}{}", prefix, text)
    }

    /// Format tags for display
    fn format_tags(&self, tags: &[String]) -> String {
        if tags.is_empty() {
            String::new()
        } else {
            let formatted_tags: Vec<String> = tags
                .iter()
                .map(|tag| self.colorize(&format!("@{}", tag), Color::Cyan))
                .collect();
            format!("{} ", formatted_tags.join(" "))
        }
    }

    /// Format step status indicator
    fn step_status_symbol(&self, status: &StepStatus) -> String {
        match status {
            StepStatus::Passed => self.colorize("✓", Color::Green),
            StepStatus::Failed => self.colorize("✗", Color::Red),
            StepStatus::Skipped => self.colorize("-", Color::Yellow),
            StepStatus::Pending => self.colorize("?", Color::Yellow),
            StepStatus::Undefined => self.colorize("U", Color::Magenta),
            StepStatus::Ambiguous => self.colorize("A", Color::Magenta),
        }
    }

    /// Format scenario status for summary
    fn scenario_status_symbol(&self, status: &ScenarioStatus) -> String {
        match status {
            ScenarioStatus::Passed => self.colorize("✓", Color::Green),
            ScenarioStatus::Failed => self.colorize("✗", Color::Red),
            ScenarioStatus::Skipped => self.colorize("-", Color::Yellow),
            ScenarioStatus::Pending => self.colorize("?", Color::Yellow),
            ScenarioStatus::Undefined => self.colorize("U", Color::Magenta),
            ScenarioStatus::Ambiguous => self.colorize("A", Color::Magenta),
        }
    }

    /// Format duration in human-readable form
    fn format_duration(&self, duration_ms: u64) -> String {
        if duration_ms < 1000 {
            format!("{}ms", duration_ms)
        } else if duration_ms < 60_000 {
            format!("{:.1}s", duration_ms as f64 / 1000.0)
        } else {
            let seconds = duration_ms / 1000;
            let minutes = seconds / 60;
            let remaining_seconds = seconds % 60;
            format!("{}m{}s", minutes, remaining_seconds)
        }
    }
}

impl<W: Write + Send + Sync> EventListener for PrettyReporter<W> {
    fn on_event(&mut self, event: &TestEvent) {
        match event {
            TestEvent::TestRunStarted { .. } => {
                if self.verbose {
                    let text = self.format_bold("Test run started");
                    let _ = writeln!(self.writer, "{}", text);
                    let _ = writeln!(self.writer);
                }
            }

            TestEvent::FeatureStarted {
                name,
                file_path,
                tags,
                ..
            } => {
                let _ = writeln!(self.writer);
                let tags_str = self.format_tags(tags);
                let feature_title = self.format_bold(&format!("Feature: {}", name));
                let _ = writeln!(self.writer, "{}{}", tags_str, feature_title);

                if self.verbose {
                    let file_info = self.colorize(&format!("  # {}", file_path), Color::Blue);
                    let _ = writeln!(self.writer, "{}", file_info);
                }

                self.current_feature = Some(name.clone());
            }

            TestEvent::ScenarioStarted {
                scenario_id,
                name,
                tags,
                outline_info,
                ..
            } => {
                let _ = writeln!(self.writer);
                let tags_str = self.format_tags(tags);

                let scenario_type = if outline_info.is_some() {
                    "Scenario Outline"
                } else {
                    "Scenario"
                };

                let scenario_title = match outline_info {
                    Some(info) => format!(
                        "{}: {} (Example #{})",
                        scenario_type,
                        name,
                        info.row_index + 1
                    ),
                    None => format!("{}: {}", scenario_type, name),
                };

                let _ = self.write_line(1, &format!("{}{}", tags_str, scenario_title));

                self.current_scenario = Some(scenario_id.clone());
                self.scenario_step_count = 0;
            }

            TestEvent::StepStarted { name, keyword, .. } => {
                // For real-time output, we could show step as it starts
                if self.verbose {
                    let step_line = format!("{} {}", keyword, name);
                    let _ = self.write_line(2, &format!("⏳ {}", step_line));
                }
            }

            TestEvent::StepFinished { step_result, .. } => {
                let symbol = self.step_status_symbol(&step_result.status);
                let step_line = format!("{} {}", step_result.keyword, step_result.name);
                let duration_str = if self.verbose && step_result.duration_ms() > 0 {
                    format!(" ({})", self.format_duration(step_result.duration_ms()))
                } else {
                    String::new()
                };

                let _ = self.write_line(2, &format!("{} {}{}", symbol, step_line, duration_str));

                // Show error details for failed steps
                if let Some(error) = &step_result.error {
                    let _ = self.write_line(
                        3,
                        &self.colorize(&format!("Error: {}", error.message), Color::Red),
                    );

                    if self.verbose {
                        if let Some(location) = &error.location {
                            let location_str =
                                format!("at {}:{}", location.file_path, location.line_number);
                            let _ = self.write_line(3, &self.colorize(&location_str, Color::Blue));
                        }
                    }
                }

                // Show attachments
                for attachment in &step_result.attachments {
                    let default_name = "attachment".to_string();
                    let attachment_name = attachment.name.as_ref().unwrap_or(&default_name);
                    let attachment_info =
                        format!("📎 {} ({})", attachment_name, attachment.media_type);
                    let _ = self.write_line(3, &self.colorize(&attachment_info, Color::Blue));
                }

                self.scenario_step_count += 1;
            }

            TestEvent::ScenarioFinished {
                scenario_result, ..
            } => {
                let status_symbol = self.scenario_status_symbol(&scenario_result.status);
                let duration_str = if scenario_result.duration_ms() > 0 {
                    format!(" ({})", self.format_duration(scenario_result.duration_ms()))
                } else {
                    String::new()
                };

                if self.verbose || !scenario_result.status.is_passed() {
                    let result_line = format!(
                        "{} {} steps{}",
                        status_symbol,
                        scenario_result.steps.len(),
                        duration_str
                    );
                    let _ = self.write_line(2, &result_line);
                }

                // Track failed scenarios for summary
                if !scenario_result.status.is_passed() {
                    self.failed_scenarios.push(format!(
                        "{}:{} - {}",
                        scenario_result.id.feature_path,
                        scenario_result.id.scenario_line,
                        scenario_result.name
                    ));
                }

                self.current_scenario = None;
            }

            TestEvent::FeatureFinished { feature_result, .. } => {
                let (passed, failed, total) = feature_result.scenario_counts();
                let status_symbol = match feature_result.status {
                    FeatureStatus::Passed => self.colorize("✓", Color::Green),
                    FeatureStatus::Failed => self.colorize("✗", Color::Red),
                    _ => self.colorize("?", Color::Yellow),
                };

                let summary = format!(
                    "{} {} scenarios ({} passed, {} failed)",
                    status_symbol, total, passed, failed
                );

                if self.verbose || failed > 0 {
                    let _ = self.write_line(1, &summary);
                }

                self.current_feature = None;
            }

            TestEvent::TestRunFinished { run_result, .. } => {
                let _ = writeln!(self.writer);
                let title = self.format_underline(&self.format_bold("Test Results:"));
                let _ = writeln!(self.writer, "{}", title);

                // Overall summary
                let total_scenarios = run_result.total_scenarios;
                let passed_scenarios = run_result.passed_scenarios;
                let failed_scenarios = run_result.failed_scenarios;
                let pass_percentage = run_result.pass_percentage();

                let _ = writeln!(
                    self.writer,
                    "Scenarios: {} total, {} passed, {} failed ({:.1}% pass rate)",
                    total_scenarios,
                    self.colorize(&passed_scenarios.to_string(), Color::Green),
                    if failed_scenarios > 0 {
                        self.colorize(&failed_scenarios.to_string(), Color::Red)
                    } else {
                        failed_scenarios.to_string()
                    },
                    pass_percentage
                );

                let _ = writeln!(
                    self.writer,
                    "Steps: {} total, {} passed, {} failed, {} skipped, {} undefined",
                    run_result.total_steps,
                    self.colorize(&run_result.passed_steps.to_string(), Color::Green),
                    if run_result.failed_steps > 0 {
                        self.colorize(&run_result.failed_steps.to_string(), Color::Red)
                    } else {
                        run_result.failed_steps.to_string()
                    },
                    run_result.skipped_steps,
                    if run_result.undefined_steps > 0 {
                        self.colorize(&run_result.undefined_steps.to_string(), Color::Magenta)
                    } else {
                        run_result.undefined_steps.to_string()
                    }
                );

                if run_result.flaky_scenarios > 0 {
                    let _ = writeln!(
                        self.writer,
                        "Flaky scenarios: {} (passed after retry)",
                        self.colorize(&run_result.flaky_scenarios.to_string(), Color::Yellow)
                    );
                }

                let duration_str = self.format_duration(run_result.duration_ms());
                let _ = writeln!(self.writer, "Duration: {}", duration_str);

                // Show failed scenario details
                if !self.failed_scenarios.is_empty() {
                    let _ = writeln!(self.writer);
                    let _ = writeln!(
                        self.writer,
                        "{}",
                        self.colorize("Failed scenarios:", Color::Red)
                    );
                    for failed in &self.failed_scenarios {
                        let _ = writeln!(self.writer, "  {}", self.colorize(failed, Color::Red));
                    }
                }

                // Final result
                let _ = writeln!(self.writer);
                if run_result.is_success() {
                    let _ = writeln!(
                        self.writer,
                        "{}",
                        self.colorize("✓ All tests passed!", Color::Green)
                    );
                } else {
                    let _ = writeln!(
                        self.writer,
                        "{}",
                        self.colorize("✗ Some tests failed", Color::Red)
                    );
                }
            }

            TestEvent::HookStarted {
                hook_name,
                hook_type,
                ..
            } => {
                if self.verbose {
                    let hook_desc = match hook_type {
                        HookType::BeforeAll => "BeforeAll hook",
                        HookType::AfterAll => "AfterAll hook",
                        HookType::Before => "Before hook",
                        HookType::After => "After hook",
                    };
                    let _ = self.write_line(1, &format!("🎣 {} ({})", hook_name, hook_desc));
                }
            }

            TestEvent::HookFinished {
                hook_name,
                hook_type,
                status,
                error,
                ..
            } => {
                if self.verbose || *status != HookStatus::Passed {
                    let status_symbol = match status {
                        HookStatus::Passed => self.colorize("✓", Color::Green),
                        HookStatus::Failed => self.colorize("✗", Color::Red),
                        HookStatus::Skipped => self.colorize("-", Color::Yellow),
                    };

                    let hook_desc = match hook_type {
                        HookType::BeforeAll => "BeforeAll",
                        HookType::AfterAll => "AfterAll",
                        HookType::Before => "Before",
                        HookType::After => "After",
                    };

                    let _ = self.write_line(
                        1,
                        &format!("{} {} hook ({})", status_symbol, hook_desc, hook_name),
                    );

                    if let Some(error_msg) = error {
                        let _ = self.write_line(
                            2,
                            &self.colorize(&format!("Error: {}", error_msg), Color::Red),
                        );
                    }
                }
            }

            // Other events are not directly displayed in Pretty format
            _ => {}
        }

        // Ensure output is flushed for real-time display
        let _ = self.writer.flush();
    }

    fn name(&self) -> &str {
        "PrettyReporter"
    }
}

/// Progress reporter with single-line progress display
///
/// Features:
/// - Single-line progress bar showing current progress
/// - Real-time scenario counts (passed/failed/total)
/// - Dots or characters for each scenario result
/// - Final summary with failure details
/// - Compact output suitable for CI/CD logs
pub struct ProgressReporter<W: Write> {
    writer: W,
    total_scenarios: usize,
    completed_scenarios: usize,
    passed_scenarios: usize,
    failed_scenarios: usize,
    current_feature: Option<String>,
    current_scenario: Option<TestId>,
    failed_scenario_details: Vec<String>,
    show_colors: bool,
    use_dots: bool,
}

impl<W: Write> ProgressReporter<W> {
    /// Create new Progress reporter writing to the given writer
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            total_scenarios: 0,
            completed_scenarios: 0,
            passed_scenarios: 0,
            failed_scenarios: 0,
            current_feature: None,
            current_scenario: None,
            failed_scenario_details: Vec::new(),
            show_colors: true,
            use_dots: true,
        }
    }

    /// Create Progress reporter with custom settings
    pub fn with_options(writer: W, show_colors: bool, use_dots: bool) -> Self {
        Self {
            writer,
            total_scenarios: 0,
            completed_scenarios: 0,
            passed_scenarios: 0,
            failed_scenarios: 0,
            current_feature: None,
            current_scenario: None,
            failed_scenario_details: Vec::new(),
            show_colors,
            use_dots,
        }
    }

    /// Format text with color if colors are enabled
    fn colorize(&self, text: &str, color: Color) -> String {
        if self.show_colors {
            Paint::new(text).fg(color).to_string()
        } else {
            text.to_string()
        }
    }

    /// Update progress line (overwrites current line)
    fn update_progress_line(&mut self) -> io::Result<()> {
        let progress_text = if self.total_scenarios > 0 {
            let percentage =
                (self.completed_scenarios as f32 / self.total_scenarios as f32) * 100.0;
            format!(
                "Progress: {:.1}% ({}/{}) - {} passed, {} failed",
                percentage,
                self.completed_scenarios,
                self.total_scenarios,
                self.colorize(&self.passed_scenarios.to_string(), Color::Green),
                if self.failed_scenarios > 0 {
                    self.colorize(&self.failed_scenarios.to_string(), Color::Red)
                } else {
                    self.failed_scenarios.to_string()
                }
            )
        } else {
            "Starting test run...".to_string()
        };

        // Clear current line and write new progress
        write!(self.writer, "\r\x1b[K{}", progress_text)?;
        self.writer.flush()
    }

    /// Write a scenario result dot/character
    fn write_scenario_dot(&mut self, passed: bool) -> io::Result<()> {
        if self.use_dots {
            let symbol = if passed {
                self.colorize(".", Color::Green)
            } else {
                self.colorize("F", Color::Red)
            };
            write!(self.writer, "{}", symbol)?;

            // Add newline every 80 characters for readability
            if self.completed_scenarios % 80 == 0 {
                writeln!(self.writer)?;
            }
        }
        self.writer.flush()
    }

    /// Format duration in human-readable form
    fn format_duration(&self, duration_ms: u64) -> String {
        if duration_ms < 1000 {
            format!("{}ms", duration_ms)
        } else if duration_ms < 60_000 {
            format!("{:.1}s", duration_ms as f64 / 1000.0)
        } else {
            let seconds = duration_ms / 1000;
            let minutes = seconds / 60;
            let remaining_seconds = seconds % 60;
            format!("{}m{}s", minutes, remaining_seconds)
        }
    }
}

impl<W: Write + Send + Sync> EventListener for ProgressReporter<W> {
    fn on_event(&mut self, event: &TestEvent) {
        match event {
            TestEvent::TestRunStarted { .. } => {
                let _ = writeln!(self.writer, "Starting cucumber-gherkin test run...");
                self.total_scenarios = 0;
                self.completed_scenarios = 0;
                self.passed_scenarios = 0;
                self.failed_scenarios = 0;
                self.failed_scenario_details.clear();
            }

            TestEvent::FeatureStarted { name, .. } => {
                self.current_feature = Some(name.clone());
                // We don't know total scenarios yet, will be updated dynamically
            }

            TestEvent::ScenarioStarted { scenario_id, .. } => {
                self.current_scenario = Some(scenario_id.clone());
                self.total_scenarios += 1;
                let _ = self.update_progress_line();
            }

            TestEvent::ScenarioFinished {
                scenario_result, ..
            } => {
                self.completed_scenarios += 1;

                if scenario_result.status.is_passed() {
                    self.passed_scenarios += 1;
                    let _ = self.write_scenario_dot(true);
                } else {
                    self.failed_scenarios += 1;
                    let _ = self.write_scenario_dot(false);

                    // Collect failed scenario info for summary
                    let failure_info = format!(
                        "{}:{} - {} ({})",
                        scenario_result.id.feature_path,
                        scenario_result.id.scenario_line,
                        scenario_result.name,
                        match scenario_result.first_failure() {
                            Some(failed_step) => {
                                if let Some(error) = &failed_step.error {
                                    format!(
                                        "{} {}: {}",
                                        failed_step.keyword, failed_step.name, error.message
                                    )
                                } else {
                                    format!(
                                        "{} {}: Step failed",
                                        failed_step.keyword, failed_step.name
                                    )
                                }
                            }
                            None => "Unknown failure".to_string(),
                        }
                    );
                    self.failed_scenario_details.push(failure_info);
                }

                let _ = self.update_progress_line();
                self.current_scenario = None;
            }

            TestEvent::FeatureFinished { .. } => {
                self.current_feature = None;
            }

            TestEvent::TestRunFinished { run_result, .. } => {
                // Ensure we're on a new line after progress updates
                let _ = writeln!(self.writer);
                let _ = writeln!(self.writer);

                // Final summary
                let total_scenarios = run_result.total_scenarios;
                let passed_scenarios = run_result.passed_scenarios;
                let failed_scenarios = run_result.failed_scenarios;
                let pass_percentage = run_result.pass_percentage();

                let summary_line = format!(
                    "Finished: {} scenarios ({} passed, {} failed) - {:.1}% pass rate in {}",
                    total_scenarios,
                    self.colorize(&passed_scenarios.to_string(), Color::Green),
                    if failed_scenarios > 0 {
                        self.colorize(&failed_scenarios.to_string(), Color::Red)
                    } else {
                        failed_scenarios.to_string()
                    },
                    pass_percentage,
                    self.format_duration(run_result.duration_ms())
                );
                let _ = writeln!(self.writer, "{}", summary_line);

                // Show step summary
                let _ = writeln!(
                    self.writer,
                    "Steps: {} total, {} passed, {} failed, {} skipped, {} undefined",
                    run_result.total_steps,
                    self.colorize(&run_result.passed_steps.to_string(), Color::Green),
                    if run_result.failed_steps > 0 {
                        self.colorize(&run_result.failed_steps.to_string(), Color::Red)
                    } else {
                        run_result.failed_steps.to_string()
                    },
                    run_result.skipped_steps,
                    if run_result.undefined_steps > 0 {
                        self.colorize(&run_result.undefined_steps.to_string(), Color::Magenta)
                    } else {
                        run_result.undefined_steps.to_string()
                    }
                );

                // Show failed scenarios details if any
                if !self.failed_scenario_details.is_empty() {
                    let _ = writeln!(self.writer);
                    let _ = writeln!(
                        self.writer,
                        "{}",
                        self.colorize("Failed scenarios:", Color::Red)
                    );
                    for failure in &self.failed_scenario_details {
                        let _ = writeln!(self.writer, "  {}", self.colorize(failure, Color::Red));
                    }
                }

                // Final result
                let _ = writeln!(self.writer);
                if run_result.is_success() {
                    let _ = writeln!(
                        self.writer,
                        "{}",
                        self.colorize("✓ All tests passed!", Color::Green)
                    );
                } else {
                    let _ = writeln!(
                        self.writer,
                        "{}",
                        self.colorize("✗ Some tests failed", Color::Red)
                    );
                }
            }

            // Progress reporter ignores most other events for clean output
            _ => {}
        }

        // Ensure output is flushed for real-time display
        let _ = self.writer.flush();
    }

    fn name(&self) -> &str {
        "ProgressReporter"
    }
}

/// JUnit XML reporter for CI integration
///
/// Features:
/// - Generates JUnit XML format compatible with most CI systems
/// - One test suite per feature file
/// - One test case per scenario (including Scenario Outline examples)
/// - Proper failure messages with stack traces
/// - System output and error capture
/// - Timing information for performance analysis
pub struct JunitReporter<W: Write> {
    writer: W,
    features: Vec<FeatureResult>,
    current_feature: Option<FeatureResult>,
    current_scenario: Option<ScenarioResult>,
    current_run_result: Option<RunResult>,
}

impl<W: Write> JunitReporter<W> {
    /// Create new JUnit reporter writing to the given writer
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            features: Vec::new(),
            current_feature: None,
            current_scenario: None,
            current_run_result: None,
        }
    }

    /// Escape XML special characters
    fn escape_xml(&self, text: &str) -> String {
        text.chars()
            .map(|c| match c {
                '&' => "&amp;".to_string(),
                '<' => "&lt;".to_string(),
                '>' => "&gt;".to_string(),
                '"' => "&quot;".to_string(),
                '\'' => "&#39;".to_string(),
                '\n' => "&#10;".to_string(),
                '\r' => "&#13;".to_string(),
                '\t' => "&#9;".to_string(),
                c if c.is_control() => format!("&#{};", c as u32),
                c => c.to_string(),
            })
            .collect()
    }

    /// Format duration in seconds with millisecond precision
    fn format_duration_seconds(&self, duration_ms: u64) -> String {
        format!("{:.3}", duration_ms as f64 / 1000.0)
    }

    /// Generate JUnit XML content
    fn generate_junit_xml(&self) -> String {
        let mut xml = String::new();

        // XML header
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');

        let run_result = match &self.current_run_result {
            Some(result) => result,
            None => return xml, // No test run data
        };

        // Root testsuites element
        xml.push_str(&format!(
            r#"<testsuites name="cucumber-gherkin" tests="{}" failures="{}" errors="{}" time="{}">"#,
            run_result.total_scenarios,
            run_result.failed_scenarios,
            run_result.undefined_steps + run_result.pending_steps, // Count undefined/pending as errors
            self.format_duration_seconds(run_result.duration_ms())
        ));
        xml.push('\n');

        // Add each feature as a test suite
        for feature in &self.features {
            let (passed, failed, total) = feature.scenario_counts();
            let errors = feature
                .scenarios
                .iter()
                .filter(|s| {
                    matches!(
                        s.status,
                        ScenarioStatus::Undefined
                            | ScenarioStatus::Pending
                            | ScenarioStatus::Ambiguous
                    )
                })
                .count();

            xml.push_str(&format!(
                r#"  <testsuite name="{}" tests="{}" failures="{}" errors="{}" time="{}" file="{}">"#,
                self.escape_xml(&feature.name),
                total,
                failed - errors, // JUnit distinguishes failures from errors
                errors,
                self.format_duration_seconds(feature.duration_ms()),
                self.escape_xml(&feature.file_path)
            ));
            xml.push('\n');

            // Add each scenario as a test case
            for scenario in &feature.scenarios {
                let class_name = format!(
                    "{}.{}",
                    feature.file_path.replace('/', ".").replace(".feature", ""),
                    feature.name.replace(' ', "_")
                );
                let test_name = match &scenario.outline_info {
                    Some(info) => format!("{} [Example {}]", scenario.name, info.row_index + 1),
                    None => scenario.name.clone(),
                };

                xml.push_str(&format!(
                    r#"    <testcase name="{}" classname="{}" time="{}">"#,
                    self.escape_xml(&test_name),
                    self.escape_xml(&class_name),
                    self.format_duration_seconds(scenario.duration_ms())
                ));
                xml.push('\n');

                // Add failure information if scenario failed
                match scenario.status {
                    ScenarioStatus::Failed => {
                        if let Some(failed_step) = scenario.first_failure() {
                            let message = if let Some(error) = &failed_step.error {
                                format!(
                                    "{} {}: {}",
                                    failed_step.keyword, failed_step.name, error.message
                                )
                            } else {
                                format!("{} {}: Step failed", failed_step.keyword, failed_step.name)
                            };

                            xml.push_str(&format!(
                                r#"      <failure message="{}" type="StepFailure">{}</failure>"#,
                                self.escape_xml(&message),
                                self.escape_xml(&message)
                            ));
                            xml.push('\n');
                        }
                    }
                    ScenarioStatus::Undefined
                    | ScenarioStatus::Pending
                    | ScenarioStatus::Ambiguous => {
                        let error_type = match scenario.status {
                            ScenarioStatus::Undefined => "UndefinedStep",
                            ScenarioStatus::Pending => "PendingStep",
                            ScenarioStatus::Ambiguous => "AmbiguousStep",
                            _ => "UnknownError",
                        };
                        let message = format!("Scenario has {} steps", error_type.to_lowercase());

                        xml.push_str(&format!(
                            r#"      <error message="{}" type="{}">{}</error>"#,
                            self.escape_xml(&message),
                            error_type,
                            self.escape_xml(&message)
                        ));
                        xml.push('\n');
                    }
                    _ => {} // Passed scenarios don't need additional elements
                }

                // Add system output (step details)
                if !scenario.steps.is_empty() {
                    let mut system_out = String::new();
                    for step in &scenario.steps {
                        system_out.push_str(&format!("  {} {}\n", step.keyword, step.name));
                        if let Some(error) = &step.error {
                            system_out.push_str(&format!("    Error: {}\n", error.message));
                        }
                    }

                    if !system_out.trim().is_empty() {
                        xml.push_str(&format!(
                            r#"      <system-out>{}</system-out>"#,
                            self.escape_xml(&system_out)
                        ));
                        xml.push('\n');
                    }
                }

                // Add system error for failed scenarios
                if scenario.status == ScenarioStatus::Failed {
                    if let Some(failed_step) = scenario.first_failure() {
                        if let Some(error) = &failed_step.error {
                            let system_err = format!(
                                "Step failed: {} {}\nError: {}\nType: {}",
                                failed_step.keyword,
                                failed_step.name,
                                error.message,
                                error.error_type
                            );

                            xml.push_str(&format!(
                                r#"      <system-err>{}</system-err>"#,
                                self.escape_xml(&system_err)
                            ));
                            xml.push('\n');
                        }
                    }
                }

                xml.push_str("    </testcase>\n");
            }

            xml.push_str("  </testsuite>\n");
        }

        xml.push_str("</testsuites>\n");
        xml
    }
}

impl<W: Write + Send + Sync> EventListener for JunitReporter<W> {
    fn on_event(&mut self, event: &TestEvent) {
        match event {
            TestEvent::TestRunStarted { .. } => {
                // Initialize state
                self.features.clear();
                self.current_feature = None;
                self.current_scenario = None;
                self.current_run_result = None;
            }

            TestEvent::FeatureStarted {
                feature_id,
                name,
                file_path,
                tags,
                ..
            } => {
                self.current_feature = Some(FeatureResult::new(
                    feature_id.clone(),
                    name.clone(),
                    file_path.clone(),
                    tags.clone(),
                ));
            }

            TestEvent::ScenarioStarted {
                scenario_id,
                name,
                tags,
                outline_info,
                ..
            } => {
                self.current_scenario = Some(ScenarioResult::new(
                    scenario_id.clone(),
                    name.clone(),
                    tags.clone(),
                ));

                if let Some(scenario) = &mut self.current_scenario {
                    scenario.outline_info = outline_info.clone();
                }
            }

            TestEvent::ScenarioFinished {
                scenario_result, ..
            } => {
                if let Some(ref mut feature) = self.current_feature {
                    feature.scenarios.push(scenario_result.clone());
                    // Update feature status
                    feature.status = FeatureStatus::from_scenarios(&feature.scenarios);
                }
                self.current_scenario = None;
            }

            TestEvent::FeatureFinished { feature_result, .. } => {
                self.features.push(feature_result.clone());
                self.current_feature = None;
            }

            TestEvent::TestRunFinished { run_result, .. } => {
                self.current_run_result = Some(run_result.clone());

                // Generate and write JUnit XML
                let junit_xml = self.generate_junit_xml();
                let _ = write!(self.writer, "{}", junit_xml);
                let _ = self.writer.flush();
            }

            // JUnit reporter collects data but doesn't output during execution
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "JunitReporter"
    }
}

/// JSON reporter for machine-readable output
///
/// Features:
/// - Structured JSON output with complete test run data
/// - Real-time streaming of test events (NDJSON format option)
/// - Rich metadata including timing, tags, and error details
/// - Compatible with CI dashboards and analytics tools
/// - Stable schema for tooling integration
#[cfg(feature = "serde")]
pub struct JsonReporter<W: Write> {
    writer: W,
    current_run_result: Option<RunResult>,
    stream_mode: bool, // If true, outputs NDJSON events; if false, outputs final result only
}

#[cfg(feature = "serde")]
impl<W: Write> JsonReporter<W> {
    /// Create new JSON reporter writing final result only
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            current_run_result: None,
            stream_mode: false,
        }
    }

    /// Create new JSON reporter with streaming mode
    /// In streaming mode, outputs NDJSON (newline-delimited JSON) events in real-time
    pub fn with_streaming(writer: W) -> Self {
        Self {
            writer,
            current_run_result: None,
            stream_mode: true,
        }
    }

    /// Write a JSON event (for streaming mode)
    fn write_event(&mut self, event: &TestEvent) -> std::io::Result<()> {
        if self.stream_mode {
            // Convert TestEvent to JSON and write as NDJSON line
            match serde_json::to_string(event) {
                Ok(json_line) => {
                    writeln!(self.writer, "{}", json_line)?;
                    self.writer.flush()
                }
                Err(e) => {
                    eprintln!("Warning: Failed to serialize event to JSON: {}", e);
                    Ok(())
                }
            }
        } else {
            Ok(()) // Non-streaming mode doesn't output individual events
        }
    }
}

#[cfg(feature = "serde")]
impl<W: Write + Send + Sync> EventListener for JsonReporter<W> {
    fn on_event(&mut self, event: &TestEvent) {
        // Always write events in streaming mode
        let _ = self.write_event(event);

        // Also collect final result for both modes
        match event {
            TestEvent::TestRunStarted { .. } => {
                self.current_run_result = None;
            }

            TestEvent::TestRunFinished { run_result, .. } => {
                self.current_run_result = Some(run_result.clone());

                // In non-streaming mode, output complete result at end
                if !self.stream_mode {
                    match serde_json::to_string_pretty(run_result) {
                        Ok(json_output) => {
                            let _ = writeln!(self.writer, "{}", json_output);
                            let _ = self.writer.flush();
                        }
                        Err(e) => {
                            eprintln!("Error: Failed to serialize run result to JSON: {}", e);
                        }
                    }
                }
            }

            // For streaming mode, all events are written by write_event()
            _ => {}
        }
    }

    fn name(&self) -> &str {
        if self.stream_mode {
            "JsonStreamReporter"
        } else {
            "JsonReporter"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::*;
    use std::time::Duration;
    use std::time::SystemTime;

    #[test]
    fn test_pretty_reporter_creation() {
        let output = Vec::new();
        let reporter = PrettyReporter::new(output);
        assert_eq!(reporter.name(), "PrettyReporter");
    }

    #[test]
    fn test_pretty_reporter_test_run_events() {
        let output = Vec::new();
        let mut reporter = PrettyReporter::with_options(output, false, true); // No colors, verbose

        // Test run started
        reporter.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: std::collections::HashMap::new(),
        });

        // Test run finished
        let run_result = RunResult {
            features: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_secs(5),
            build_metadata: std::collections::HashMap::new(),
            total_scenarios: 2,
            passed_scenarios: 1,
            failed_scenarios: 1,
            flaky_scenarios: 0,
            total_steps: 4,
            passed_steps: 2,
            failed_steps: 1,
            skipped_steps: 1,
            undefined_steps: 0,
            pending_steps: 0,
        };

        reporter.on_event(&TestEvent::TestRunFinished {
            end_time: SystemTime::now(),
            run_result,
        });

        let output_str = String::from_utf8(reporter.writer).unwrap();
        assert!(output_str.contains("Test run started"));
        assert!(output_str.contains("Test Results:"));
        assert!(output_str.contains("2 total, 1 passed, 1 failed"));
        assert!(output_str.contains("Some tests failed"));
    }

    #[test]
    fn test_pretty_reporter_feature_scenario_events() {
        let output = Vec::new();
        let mut reporter = PrettyReporter::with_options(output, false, false); // No colors, not verbose

        // Feature started
        reporter.on_event(&TestEvent::FeatureStarted {
            feature_id: "feature1".to_string(),
            name: "Test Feature".to_string(),
            file_path: "test.feature".to_string(),
            tags: vec!["smoke".to_string(), "fast".to_string()],
            start_time: SystemTime::now(),
        });

        // Scenario started
        let scenario_id = TestId::new("test.feature".to_string(), 5, "Test Scenario".to_string());
        reporter.on_event(&TestEvent::ScenarioStarted {
            scenario_id: scenario_id.clone(),
            name: "Test Scenario".to_string(),
            tags: vec!["important".to_string()],
            outline_info: None,
            start_time: SystemTime::now(),
        });

        // Step finished
        let step_result = StepResult {
            id: "step1".to_string(),
            name: "I do something".to_string(),
            keyword: "Given".to_string(),
            location: None,
            status: StepStatus::Passed,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(100),
            error: None,
            attachments: vec![],
            attempt: 1,
        };

        reporter.on_event(&TestEvent::StepFinished {
            step_id: "step1".to_string(),
            scenario_id: scenario_id.clone(),
            step_result,
        });

        // Scenario finished
        let scenario_result = ScenarioResult {
            id: scenario_id,
            name: "Test Scenario".to_string(),
            tags: vec!["important".to_string()],
            outline_info: None,
            steps: vec![],
            status: ScenarioStatus::Passed,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(150),
            retry_count: 0,
            attachments: vec![],
        };

        reporter.on_event(&TestEvent::ScenarioFinished {
            scenario_id: TestId::new("test.feature".to_string(), 5, "Test Scenario".to_string()),
            scenario_result,
        });

        let output_str = String::from_utf8(reporter.writer).unwrap();
        assert!(output_str.contains("Feature: Test Feature"));
        assert!(output_str.contains("@smoke @fast"));
        assert!(output_str.contains("Scenario: Test Scenario"));
        assert!(output_str.contains("@important"));
        assert!(output_str.contains("Given I do something"));
    }

    #[test]
    fn test_duration_formatting() {
        let output = Vec::new();
        let reporter = PrettyReporter::new(output);

        assert_eq!(reporter.format_duration(500), "500ms");
        assert_eq!(reporter.format_duration(1500), "1.5s");
        assert_eq!(reporter.format_duration(65000), "1m5s");
        assert_eq!(reporter.format_duration(125000), "2m5s");
    }

    #[test]
    fn test_tag_formatting() {
        let output = Vec::new();
        let reporter = PrettyReporter::with_options(output, false, false);

        assert_eq!(reporter.format_tags(&[]), "");
        assert_eq!(reporter.format_tags(&["smoke".to_string()]), "@smoke ");
        assert_eq!(
            reporter.format_tags(&["smoke".to_string(), "fast".to_string()]),
            "@smoke @fast "
        );
    }

    #[test]
    fn test_step_status_symbols() {
        let output = Vec::new();
        let reporter = PrettyReporter::with_options(output, false, false);

        assert_eq!(reporter.step_status_symbol(&StepStatus::Passed), "✓");
        assert_eq!(reporter.step_status_symbol(&StepStatus::Failed), "✗");
        assert_eq!(reporter.step_status_symbol(&StepStatus::Skipped), "-");
        assert_eq!(reporter.step_status_symbol(&StepStatus::Pending), "?");
        assert_eq!(reporter.step_status_symbol(&StepStatus::Undefined), "U");
        assert_eq!(reporter.step_status_symbol(&StepStatus::Ambiguous), "A");
    }

    #[test]
    fn test_progress_reporter_creation() {
        let output = Vec::new();
        let reporter = ProgressReporter::new(output);
        assert_eq!(reporter.name(), "ProgressReporter");
        assert_eq!(reporter.total_scenarios, 0);
        assert_eq!(reporter.completed_scenarios, 0);
        assert!(reporter.show_colors);
        assert!(reporter.use_dots);
    }

    #[test]
    fn test_progress_reporter_options() {
        let output = Vec::new();
        let reporter = ProgressReporter::with_options(output, false, false);
        assert!(!reporter.show_colors);
        assert!(!reporter.use_dots);
    }

    #[test]
    fn test_progress_reporter_test_run_lifecycle() {
        let output = Vec::new();
        let mut reporter = ProgressReporter::with_options(output, false, true); // No colors, use dots

        // Test run started
        reporter.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: std::collections::HashMap::new(),
        });

        // Scenario started
        let scenario_id = TestId::new("test.feature".to_string(), 5, "Test Scenario".to_string());
        reporter.on_event(&TestEvent::ScenarioStarted {
            scenario_id: scenario_id.clone(),
            name: "Test Scenario".to_string(),
            tags: vec![],
            outline_info: None,
            start_time: SystemTime::now(),
        });

        // Scenario finished (passed)
        let scenario_result = ScenarioResult {
            id: scenario_id.clone(),
            name: "Test Scenario".to_string(),
            tags: vec![],
            outline_info: None,
            steps: vec![
                StepResult::new(
                    "step1".to_string(),
                    "I do something".to_string(),
                    "Given".to_string(),
                )
                .with_status(StepStatus::Passed),
            ],
            status: ScenarioStatus::Passed,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(100),
            retry_count: 0,
            attachments: vec![],
        };

        reporter.on_event(&TestEvent::ScenarioFinished {
            scenario_id: scenario_id.clone(),
            scenario_result,
        });

        // Test run finished
        let run_result = RunResult {
            features: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_secs(2),
            build_metadata: std::collections::HashMap::new(),
            total_scenarios: 1,
            passed_scenarios: 1,
            failed_scenarios: 0,
            flaky_scenarios: 0,
            total_steps: 1,
            passed_steps: 1,
            failed_steps: 0,
            skipped_steps: 0,
            undefined_steps: 0,
            pending_steps: 0,
        };

        reporter.on_event(&TestEvent::TestRunFinished {
            end_time: SystemTime::now(),
            run_result,
        });

        let output_str = String::from_utf8(reporter.writer).unwrap();
        dbg!("Progress reporter output: {}", &output_str);

        assert!(output_str.contains("Starting cucumber-gherkin test run"));
        assert!(output_str.contains("Finished: 1 scenarios"));
        assert!(output_str.contains("1 passed, 0 failed"));
        assert!(output_str.contains("100.0% pass rate"));
        assert!(output_str.contains("All tests passed"));

        // Check internal state
        assert_eq!(reporter.total_scenarios, 1);
        assert_eq!(reporter.completed_scenarios, 1);
        assert_eq!(reporter.passed_scenarios, 1);
        assert_eq!(reporter.failed_scenarios, 0);
    }

    #[test]
    fn test_progress_reporter_failed_scenario() {
        let output = Vec::new();
        let mut reporter = ProgressReporter::with_options(output, false, true); // No colors, use dots

        // Test run started
        reporter.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: std::collections::HashMap::new(),
        });

        // Failed scenario
        let scenario_id = TestId::new(
            "test.feature".to_string(),
            10,
            "Failed Scenario".to_string(),
        );
        reporter.on_event(&TestEvent::ScenarioStarted {
            scenario_id: scenario_id.clone(),
            name: "Failed Scenario".to_string(),
            tags: vec![],
            outline_info: None,
            start_time: SystemTime::now(),
        });

        let failed_step = StepResult {
            id: "step1".to_string(),
            name: "something fails".to_string(),
            keyword: "When".to_string(),
            location: None,
            status: StepStatus::Failed,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(50),
            error: Some(crate::results::StepError {
                message: "Expected true but got false".to_string(),
                error_type: "AssertionError".to_string(),
                stack_trace: None,
                location: None,
            }),
            attachments: vec![],
            attempt: 1,
        };

        let scenario_result = ScenarioResult {
            id: scenario_id.clone(),
            name: "Failed Scenario".to_string(),
            tags: vec![],
            outline_info: None,
            steps: vec![failed_step],
            status: ScenarioStatus::Failed,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(100),
            retry_count: 0,
            attachments: vec![],
        };

        reporter.on_event(&TestEvent::ScenarioFinished {
            scenario_id: scenario_id.clone(),
            scenario_result,
        });

        // Test run finished
        let run_result = RunResult {
            features: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_secs(1),
            build_metadata: std::collections::HashMap::new(),
            total_scenarios: 1,
            passed_scenarios: 0,
            failed_scenarios: 1,
            flaky_scenarios: 0,
            total_steps: 1,
            passed_steps: 0,
            failed_steps: 1,
            skipped_steps: 0,
            undefined_steps: 0,
            pending_steps: 0,
        };

        reporter.on_event(&TestEvent::TestRunFinished {
            end_time: SystemTime::now(),
            run_result,
        });

        let output_str = String::from_utf8(reporter.writer).unwrap();
        dbg!("Progress reporter failed output: {}", &output_str);

        assert!(output_str.contains("0 passed, 1 failed"));
        assert!(output_str.contains("0.0% pass rate"));
        assert!(output_str.contains("Some tests failed"));
        assert!(output_str.contains("Failed scenarios:"));
        assert!(output_str.contains("test.feature:10 - Failed Scenario"));
        assert!(output_str.contains("Expected true but got false"));

        // Check internal state
        assert_eq!(reporter.failed_scenarios, 1);
        assert_eq!(reporter.failed_scenario_details.len(), 1);
    }

    #[test]
    fn test_junit_reporter_creation() {
        let output = Vec::new();
        let reporter = JunitReporter::new(output);
        assert_eq!(reporter.name(), "JunitReporter");
        assert_eq!(reporter.features.len(), 0);
    }

    #[test]
    fn test_junit_reporter_xml_escaping() {
        let output = Vec::new();
        let reporter = JunitReporter::new(output);

        // Test XML character escaping
        assert_eq!(reporter.escape_xml("normal text"), "normal text");
        assert_eq!(reporter.escape_xml("test & example"), "test &amp; example");
        assert_eq!(
            reporter.escape_xml("<tag>content</tag>"),
            "&lt;tag&gt;content&lt;/tag&gt;"
        );
        assert_eq!(
            reporter.escape_xml("\"quotes\" and 'apostrophes'"),
            "&quot;quotes&quot; and &#39;apostrophes&#39;"
        );
        assert_eq!(
            reporter.escape_xml("line1\nline2\ttab"),
            "line1&#10;line2&#9;tab"
        );
    }

    #[test]
    fn test_junit_reporter_duration_formatting() {
        let output = Vec::new();
        let reporter = JunitReporter::new(output);

        assert_eq!(reporter.format_duration_seconds(0), "0.000");
        assert_eq!(reporter.format_duration_seconds(500), "0.500");
        assert_eq!(reporter.format_duration_seconds(1234), "1.234");
        assert_eq!(reporter.format_duration_seconds(65000), "65.000");
    }

    #[test]
    fn test_junit_reporter_successful_test_run() {
        let output = Vec::new();
        let mut reporter = JunitReporter::new(output);

        // Test run started
        reporter.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: std::collections::HashMap::new(),
        });

        // Feature started
        reporter.on_event(&TestEvent::FeatureStarted {
            feature_id: "feature1".to_string(),
            name: "Test Feature".to_string(),
            file_path: "features/test.feature".to_string(),
            tags: vec!["smoke".to_string()],
            start_time: SystemTime::now(),
        });

        // Scenario started
        let scenario_id = TestId::new(
            "features/test.feature".to_string(),
            5,
            "Test Scenario".to_string(),
        );
        reporter.on_event(&TestEvent::ScenarioStarted {
            scenario_id: scenario_id.clone(),
            name: "Test Scenario".to_string(),
            tags: vec![],
            outline_info: None,
            start_time: SystemTime::now(),
        });

        // Scenario finished (passed)
        let scenario_result = ScenarioResult {
            id: scenario_id.clone(),
            name: "Test Scenario".to_string(),
            tags: vec![],
            outline_info: None,
            steps: vec![
                StepResult::new(
                    "step1".to_string(),
                    "I do something".to_string(),
                    "Given".to_string(),
                )
                .with_status(StepStatus::Passed),
            ],
            status: ScenarioStatus::Passed,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(100),
            retry_count: 0,
            attachments: vec![],
        };

        reporter.on_event(&TestEvent::ScenarioFinished {
            scenario_id: scenario_id.clone(),
            scenario_result: scenario_result.clone(),
        });

        // Feature finished
        let feature_result = FeatureResult::new(
            "feature1".to_string(),
            "Test Feature".to_string(),
            "features/test.feature".to_string(),
            vec!["smoke".to_string()],
        )
        .with_scenarios(vec![scenario_result])
        .with_duration(SystemTime::now(), SystemTime::now());

        reporter.on_event(&TestEvent::FeatureFinished {
            feature_id: "feature1".to_string(),
            feature_result,
        });

        // Test run finished
        let run_result = RunResult {
            features: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_secs(1),
            build_metadata: std::collections::HashMap::new(),
            total_scenarios: 1,
            passed_scenarios: 1,
            failed_scenarios: 0,
            flaky_scenarios: 0,
            total_steps: 1,
            passed_steps: 1,
            failed_steps: 0,
            skipped_steps: 0,
            undefined_steps: 0,
            pending_steps: 0,
        };

        reporter.on_event(&TestEvent::TestRunFinished {
            end_time: SystemTime::now(),
            run_result,
        });

        let output_str = String::from_utf8(reporter.writer).unwrap();
        dbg!("JUnit successful output: {}", &output_str);

        // Validate XML structure
        assert!(output_str.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(output_str.contains("<testsuites"));
        assert!(output_str.contains("name=\"cucumber-gherkin\""));
        assert!(output_str.contains("tests=\"1\""));
        assert!(output_str.contains("failures=\"0\""));
        assert!(output_str.contains("errors=\"0\""));
        assert!(output_str.contains("<testsuite name=\"Test Feature\""));
        assert!(output_str.contains("file=\"features/test.feature\""));
        assert!(output_str.contains("<testcase name=\"Test Scenario\""));
        assert!(output_str.contains("classname=\"features.test.Test_Feature\""));
        assert!(output_str.contains("</testcase>"));
        assert!(output_str.contains("</testsuite>"));
        assert!(output_str.contains("</testsuites>"));

        // Should not contain failure or error elements for successful test
        assert!(!output_str.contains("<failure"));
        assert!(!output_str.contains("<error"));
    }

    #[test]
    fn test_junit_reporter_failed_test_run() {
        let output = Vec::new();
        let mut reporter = JunitReporter::new(output);

        // Test run started
        reporter.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: std::collections::HashMap::new(),
        });

        // Feature started
        reporter.on_event(&TestEvent::FeatureStarted {
            feature_id: "feature1".to_string(),
            name: "Failed Feature".to_string(),
            file_path: "features/failed.feature".to_string(),
            tags: vec![],
            start_time: SystemTime::now(),
        });

        // Failed scenario
        let scenario_id = TestId::new(
            "features/failed.feature".to_string(),
            10,
            "Failed Scenario".to_string(),
        );
        reporter.on_event(&TestEvent::ScenarioStarted {
            scenario_id: scenario_id.clone(),
            name: "Failed Scenario".to_string(),
            tags: vec![],
            outline_info: None,
            start_time: SystemTime::now(),
        });

        let failed_step = StepResult {
            id: "step1".to_string(),
            name: "something fails".to_string(),
            keyword: "When".to_string(),
            location: None,
            status: StepStatus::Failed,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(50),
            error: Some(crate::results::StepError {
                message: "Expected <true> but got <false>".to_string(),
                error_type: "AssertionError".to_string(),
                stack_trace: None,
                location: None,
            }),
            attachments: vec![],
            attempt: 1,
        };

        let scenario_result = ScenarioResult {
            id: scenario_id.clone(),
            name: "Failed Scenario".to_string(),
            tags: vec![],
            outline_info: None,
            steps: vec![failed_step],
            status: ScenarioStatus::Failed,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(100),
            retry_count: 0,
            attachments: vec![],
        };

        reporter.on_event(&TestEvent::ScenarioFinished {
            scenario_id: scenario_id.clone(),
            scenario_result: scenario_result.clone(),
        });

        // Feature finished
        let feature_result = FeatureResult::new(
            "feature1".to_string(),
            "Failed Feature".to_string(),
            "features/failed.feature".to_string(),
            vec![],
        )
        .with_scenarios(vec![scenario_result])
        .with_duration(SystemTime::now(), SystemTime::now());

        reporter.on_event(&TestEvent::FeatureFinished {
            feature_id: "feature1".to_string(),
            feature_result,
        });

        // Test run finished
        let run_result = RunResult {
            features: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_secs(1),
            build_metadata: std::collections::HashMap::new(),
            total_scenarios: 1,
            passed_scenarios: 0,
            failed_scenarios: 1,
            flaky_scenarios: 0,
            total_steps: 1,
            passed_steps: 0,
            failed_steps: 1,
            skipped_steps: 0,
            undefined_steps: 0,
            pending_steps: 0,
        };

        reporter.on_event(&TestEvent::TestRunFinished {
            end_time: SystemTime::now(),
            run_result,
        });

        let output_str = String::from_utf8(reporter.writer).unwrap();
        dbg!("JUnit failed output: {}", &output_str);

        // Validate XML structure for failed test
        assert!(output_str.contains("tests=\"1\""));
        assert!(output_str.contains("failures=\"1\""));
        assert!(output_str.contains("errors=\"0\""));
        assert!(output_str.contains("<failure"));
        assert!(output_str.contains(
            "message=\"When something fails: Expected &lt;true&gt; but got &lt;false&gt;\""
        ));
        assert!(output_str.contains("type=\"StepFailure\""));
        assert!(output_str.contains("<system-out>"));
        assert!(output_str.contains("When something fails"));
        assert!(output_str.contains("<system-err>"));
        assert!(output_str.contains("AssertionError"));

        // XML should be properly escaped
        assert!(output_str.contains("&lt;true&gt;"));
        assert!(output_str.contains("&lt;false&gt;"));
    }

    #[test]
    fn test_junit_reporter_scenario_outline() {
        let output = Vec::new();
        let mut reporter = JunitReporter::new(output);

        // Test run started
        reporter.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: std::collections::HashMap::new(),
        });

        // Feature started
        reporter.on_event(&TestEvent::FeatureStarted {
            feature_id: "feature1".to_string(),
            name: "Outline Feature".to_string(),
            file_path: "features/outline.feature".to_string(),
            tags: vec![],
            start_time: SystemTime::now(),
        });

        // Scenario Outline example
        let outline_info = OutlineInfo {
            row_index: 1,
            parameter_values: std::collections::HashMap::from([
                ("param1".to_string(), "value1".to_string()),
                ("param2".to_string(), "value2".to_string()),
            ]),
        };

        let scenario_id = TestId::new(
            "features/outline.feature".to_string(),
            15,
            "Outline Scenario".to_string(),
        )
        .with_row_index(1);

        reporter.on_event(&TestEvent::ScenarioStarted {
            scenario_id: scenario_id.clone(),
            name: "Outline Scenario".to_string(),
            tags: vec![],
            outline_info: Some(outline_info.clone()),
            start_time: SystemTime::now(),
        });

        let scenario_result = ScenarioResult {
            id: scenario_id.clone(),
            name: "Outline Scenario".to_string(),
            tags: vec![],
            outline_info: Some(outline_info),
            steps: vec![
                StepResult::new(
                    "step1".to_string(),
                    "I use value1 and value2".to_string(),
                    "Given".to_string(),
                )
                .with_status(StepStatus::Passed),
            ],
            status: ScenarioStatus::Passed,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(150),
            retry_count: 0,
            attachments: vec![],
        };

        reporter.on_event(&TestEvent::ScenarioFinished {
            scenario_id: scenario_id.clone(),
            scenario_result: scenario_result.clone(),
        });

        // Feature finished
        let feature_result = FeatureResult::new(
            "feature1".to_string(),
            "Outline Feature".to_string(),
            "features/outline.feature".to_string(),
            vec![],
        )
        .with_scenarios(vec![scenario_result])
        .with_duration(SystemTime::now(), SystemTime::now());

        reporter.on_event(&TestEvent::FeatureFinished {
            feature_id: "feature1".to_string(),
            feature_result,
        });

        // Test run finished
        let run_result = RunResult {
            features: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_secs(1),
            build_metadata: std::collections::HashMap::new(),
            total_scenarios: 1,
            passed_scenarios: 1,
            failed_scenarios: 0,
            flaky_scenarios: 0,
            total_steps: 1,
            passed_steps: 1,
            failed_steps: 0,
            skipped_steps: 0,
            undefined_steps: 0,
            pending_steps: 0,
        };

        reporter.on_event(&TestEvent::TestRunFinished {
            end_time: SystemTime::now(),
            run_result,
        });

        let output_str = String::from_utf8(reporter.writer).unwrap();
        dbg!("JUnit outline output: {}", &output_str);

        // Validate Scenario Outline formatting
        assert!(output_str.contains("<testcase name=\"Outline Scenario [Example 2]\""));
        assert!(output_str.contains("classname=\"features.outline.Outline_Feature\""));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_json_reporter_creation() {
        let output = Vec::new();
        let reporter = JsonReporter::new(output);
        assert_eq!(reporter.name(), "JsonReporter");
        assert!(!reporter.stream_mode);

        let output2 = Vec::new();
        let streaming_reporter = JsonReporter::with_streaming(output2);
        assert_eq!(streaming_reporter.name(), "JsonStreamReporter");
        assert!(streaming_reporter.stream_mode);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_json_reporter_final_result() {
        let output = Vec::new();
        let mut reporter = JsonReporter::new(output);

        // Test run started
        reporter.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: std::collections::HashMap::new(),
        });

        // Test run finished
        let run_result = RunResult {
            features: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_secs(2),
            build_metadata: std::collections::HashMap::new(),
            total_scenarios: 1,
            passed_scenarios: 1,
            failed_scenarios: 0,
            flaky_scenarios: 0,
            total_steps: 2,
            passed_steps: 2,
            failed_steps: 0,
            skipped_steps: 0,
            undefined_steps: 0,
            pending_steps: 0,
        };

        reporter.on_event(&TestEvent::TestRunFinished {
            end_time: SystemTime::now(),
            run_result: run_result.clone(),
        });

        let output_str = String::from_utf8(reporter.writer).unwrap();
        dbg!("JSON final result output: {}", &output_str);

        // Should be valid JSON
        let parsed_result: serde_json::Value =
            serde_json::from_str(&output_str).expect("Should be valid JSON");

        // Verify structure
        assert_eq!(parsed_result["total_scenarios"], 1);
        assert_eq!(parsed_result["passed_scenarios"], 1);
        assert_eq!(parsed_result["failed_scenarios"], 0);
        assert_eq!(parsed_result["total_steps"], 2);
        assert_eq!(parsed_result["passed_steps"], 2);
        assert!(parsed_result["features"].is_array());
        assert!(parsed_result["duration"].is_object()); // Duration is serialized as {secs, nanos}
        assert!(parsed_result["build_metadata"].is_object());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_json_reporter_streaming_mode() {
        let output = Vec::new();
        let mut reporter = JsonReporter::with_streaming(output);

        // Test run started
        reporter.on_event(&TestEvent::TestRunStarted {
            start_time: SystemTime::now(),
            build_metadata: std::collections::HashMap::new(),
        });

        // Feature started
        reporter.on_event(&TestEvent::FeatureStarted {
            feature_id: "feature1".to_string(),
            name: "Test Feature".to_string(),
            file_path: "test.feature".to_string(),
            tags: vec!["smoke".to_string()],
            start_time: SystemTime::now(),
        });

        // Test run finished
        let run_result = RunResult {
            features: vec![],
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_secs(1),
            build_metadata: std::collections::HashMap::new(),
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
        };

        reporter.on_event(&TestEvent::TestRunFinished {
            end_time: SystemTime::now(),
            run_result,
        });

        let output_str = String::from_utf8(reporter.writer).unwrap();
        dbg!("JSON streaming output: {}", &output_str);

        // Should contain multiple JSON lines (NDJSON)
        let lines: Vec<&str> = output_str.lines().collect();
        assert!(lines.len() >= 3); // At least TestRunStarted, FeatureStarted, TestRunFinished

        // Each line should be valid JSON
        for line in &lines {
            if !line.trim().is_empty() {
                let parsed: serde_json::Value =
                    serde_json::from_str(line).expect("Each line should be valid JSON");
                // Should have a variant field indicating event type
                assert!(parsed.is_object());
            }
        }

        // First event should be TestRunStarted
        let first_event: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        // Note: This will depend on serde's enum representation format
        assert!(first_event.as_object().is_some());
    }
}
