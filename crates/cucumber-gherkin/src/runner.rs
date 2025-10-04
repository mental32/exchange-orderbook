use crate::gherkin::Document;
use crate::gherkin::TagOperation;
use yansi::Paint;

/// Filter for controlling which scenarios get executed
#[derive(Debug, Clone)]
pub struct RunnerFilter {
    pub tag_expression: Option<TagOperation>,
}

impl RunnerFilter {
    pub fn new() -> Self {
        Self {
            tag_expression: None,
        }
    }

    pub fn with_tags(tag_expression: TagOperation) -> Self {
        Self {
            tag_expression: Some(tag_expression),
        }
    }
}

pub struct Runner;

impl Runner {
    pub fn run(&self, document: &Document, steps: impl crate::steps::Steps) {
        let filter = RunnerFilter::new();
        self.run_with_filter(document, steps, &filter);
    }

    pub fn run_with_filter(
        &self,
        document: &Document,
        steps: impl crate::steps::Steps,
        filter: &RunnerFilter,
    ) {
        dbg!("🎯 Runner starting with filter: {:?}", filter);

        // Run top-level scenarios first
        for scenario in &document.feature.scenarios {
            if self.should_run_scenario(
                &document.feature.header.tags,
                &[],
                &scenario.header.tags,
                &[],
                filter,
            ) {
                self.run_scenario_with_backgrounds_filtered(
                    scenario,
                    &document.feature.background,
                    &None,
                    &steps,
                    filter,
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
                    self.run_scenario_with_backgrounds_filtered(
                        scenario,
                        &document.feature.background,
                        &rule.background,
                        &steps,
                        filter,
                    );
                } else {
                    dbg!(
                        "🚫 Skipping scenario '{}' due to tag filter",
                        &scenario.header.name
                    );
                }
            }
        }
    }

    fn run_scenario_with_backgrounds(
        &self,
        scenario: &crate::gherkin::Scenario,
        feature_background: &Option<crate::gherkin::Background>,
        rule_background: &Option<crate::gherkin::Background>,
        steps: &impl crate::steps::Steps,
    ) {
        self.run_scenario_with_backgrounds_filtered(
            scenario,
            feature_background,
            rule_background,
            steps,
            &RunnerFilter::new(),
        );
    }

    fn run_scenario_with_backgrounds_filtered(
        &self,
        scenario: &crate::gherkin::Scenario,
        feature_background: &Option<crate::gherkin::Background>,
        rule_background: &Option<crate::gherkin::Background>,
        steps: &impl crate::steps::Steps,
        filter: &RunnerFilter,
    ) {
        // Check if this is a Scenario Outline with Examples
        if !scenario.examples.is_empty() {
            // Run the scenario once for each row in the Examples table
            for example_set in &scenario.examples {
                // Check if this Examples block should run based on tag filtering
                // Get effective tags for this specific Examples block: feature + rule + scenario + examples
                // Note: We'll need to pass feature/rule tags from the caller
                let effective_tags =
                    self.effective_tags(&[], &[], &scenario.header.tags, &example_set.tags);

                // Apply tag filtering at the Examples block level
                if let Some(tag_expr) = &filter.tag_expression {
                    let should_run_examples = tag_expr.evaluate(&effective_tags);
                    dbg!(
                        "📊 Examples tag filter evaluation: {:?} against tags {:?} = {}",
                        tag_expr,
                        effective_tags,
                        should_run_examples
                    );
                    if !should_run_examples {
                        dbg!(
                            "🚫 Skipping Examples '{}' due to tag filter",
                            &example_set.header.name
                        );
                        continue; // Skip this entire Examples block
                    }
                } else {
                    dbg!("✅ No tag filter, running Examples block");
                }

                // For Examples, we need to get feature + rule + scenario + examples tags
                // Note: individual example rows don't have tags in Gherkin, only Examples blocks do
                for (row_index, row) in example_set.table.rows.iter().enumerate() {
                    // Include Examples name in the output
                    let examples_name = &example_set.header.name;
                    println!(
                        "{} ({} example {})",
                        scenario.header.name.bright_blue(),
                        examples_name.bright_green(),
                        row_index + 1
                    );

                    // Run feature background steps first, then rule background steps
                    self.run_backgrounds(feature_background, rule_background, steps);

                    // Run scenario steps with parameter substitution
                    for (step, _span) in scenario.steps.iter() {
                        let substituted_step =
                            self.substitute_parameters(step, &example_set.table.header, row);
                        self.run_step(&substituted_step, steps);
                    }
                }
            }
        } else {
            // Regular scenario without examples
            println!("{}", scenario.header.name.bright_blue());

            // Run feature background steps first, then rule background steps
            self.run_backgrounds(feature_background, rule_background, steps);

            // Run scenario steps
            for (step, _span) in scenario.steps.iter() {
                self.run_step(step, steps);
            }
        }
    }

    fn run_backgrounds(
        &self,
        feature_background: &Option<crate::gherkin::Background>,
        rule_background: &Option<crate::gherkin::Background>,
        steps: &impl crate::steps::Steps,
    ) {
        // Run feature background steps first (if present)
        if let Some(background) = feature_background {
            for (step, _span) in background.steps.iter() {
                self.run_step(step, steps);
            }
        }

        // Then run rule background steps (if present)
        if let Some(background) = rule_background {
            for (step, _span) in background.steps.iter() {
                self.run_step(step, steps);
            }
        }
    }

    fn substitute_parameters(
        &self,
        step: &crate::gherkin::Step,
        headers: &[String],
        values: &[String],
    ) -> crate::gherkin::Step {
        let mut text = step.text_as_string();

        // Substitute <parameter> placeholders with actual values
        for (header, value) in headers.iter().zip(values.iter()) {
            let placeholder = format!("<{}>", header);
            text = text.replace(&placeholder, value);
        }

        // Create a new step with substituted text - we'll create a single Word text part
        let text_parts = vec![crate::gherkin::StepText::Word((
            text,
            chumsky::span::SimpleSpan {
                start: 0,
                end: 0,
                context: (),
            },
        ))]
        .into_boxed_slice();

        crate::gherkin::Step {
            verb: step.verb.clone(),
            text_parts,
            doc_string: step.doc_string.clone(),
            data_table: step.data_table.clone(),
        }
    }

    fn run_step(&self, step: &crate::gherkin::Step, steps: &impl crate::steps::Steps) {
        let step_fn = steps
            .get_step_fn(&step)
            .expect("invariant: all steps must have already been checked to exist");

        use crate::steps::StepOutput as O;

        match (step_fn)(step) {
            O::Passed => {
                println!("\t{}", step.text_as_string().bright_green());
            }
            O::Failed => {
                println!("\t{}", step.text_as_string().bright_red());
                return;
            }
            O::Skipped => {
                println!("\t{}", step.text_as_string().bright_yellow());
            }
        }
    }

    /// Compute effective tags for a scenario (Feature + Rule + Scenario + Examples)
    fn effective_tags(
        &self,
        feature_tags: &[String],
        rule_tags: &[String],
        scenario_tags: &[String],
        examples_tags: &[String],
    ) -> Vec<String> {
        let mut effective = Vec::new();
        effective.extend_from_slice(feature_tags);
        effective.extend_from_slice(rule_tags);
        effective.extend_from_slice(scenario_tags);
        effective.extend_from_slice(examples_tags);

        // Remove duplicates while preserving order
        effective.sort();
        effective.dedup();

        dbg!(
            "🏷️ Effective tags computed: feature={:?} + rule={:?} + scenario={:?} + examples={:?} = {:?}",
            feature_tags,
            rule_tags,
            scenario_tags,
            examples_tags,
            &effective
        );

        effective
    }

    /// Check if a scenario should run based on the filter
    fn should_run_scenario(
        &self,
        feature_tags: &[String],
        rule_tags: &[String],
        scenario_tags: &[String],
        examples_tags: &[String],
        filter: &RunnerFilter,
    ) -> bool {
        if let Some(tag_expr) = &filter.tag_expression {
            let effective =
                self.effective_tags(feature_tags, rule_tags, scenario_tags, examples_tags);
            let should_run = tag_expr.evaluate(&effective);
            dbg!(
                "📊 Tag filter evaluation: {:?} against tags {:?} = {}",
                tag_expr,
                effective,
                should_run
            );
            should_run
        } else {
            dbg!("✅ No tag filter, running scenario");
            true
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_effective_tags_computation() {
//         let runner = Runner::new();

//         // Test with various tag combinations
//         let feature_tags = vec!["smoke".to_string(), "integration".to_string()];
//         let rule_tags = vec!["auth".to_string()];
//         let scenario_tags = vec!["fast".to_string(), "smoke".to_string()]; // Duplicate smoke tag
//         let examples_tags = vec!["api".to_string()];

//         let effective =
//             runner.effective_tags(&feature_tags, &rule_tags, &scenario_tags, &examples_tags);

//         // Should have all unique tags, sorted
//         assert_eq!(
//             effective,
//             vec!["api", "auth", "fast", "integration", "smoke"]
//         );
//     }

//     #[test]
//     fn test_should_run_scenario_no_filter() {
//         let runner = Runner::new();
//         let filter = RunnerFilter::new(); // No tag expression

//         let should_run = runner.should_run_scenario(
//             &["smoke".to_string()],
//             &[],
//             &["fast".to_string()],
//             &[],
//             &filter,
//         );
//         assert!(should_run); // Should always run with no filter
//     }

//     #[test]
//     fn test_should_run_scenario_with_tag_filter() {
//         let runner = Runner::new();

//         // Create a tag expression: @smoke
//         let tag_expr = TagOperation::Tag("smoke".to_string());
//         let filter = RunnerFilter::with_tags(tag_expr);

//         // Scenario has @smoke tag (in feature tags)
//         let should_run = runner.should_run_scenario(&["smoke".to_string()], &[], &[], &[], &filter);
//         assert!(should_run);

//         // Scenario doesn't have @smoke tag
//         let should_not_run =
//             runner.should_run_scenario(&["slow".to_string()], &[], &[], &[], &filter);
//         assert!(!should_not_run);
//     }

//     #[test]
//     fn test_should_run_scenario_complex_tag_filter() {
//         let runner = Runner::new();

//         // Create a tag expression: @smoke and not @wip
//         let tag_expr = TagOperation::And(
//             Box::new(TagOperation::Tag("smoke".to_string())),
//             Box::new(TagOperation::Not(Box::new(TagOperation::Tag(
//                 "wip".to_string(),
//             )))),
//         );
//         let filter = RunnerFilter::with_tags(tag_expr);

//         // Scenario has @smoke but not @wip - should run
//         let should_run = runner.should_run_scenario(
//             &["smoke".to_string()],
//             &[],
//             &["fast".to_string()],
//             &[],
//             &filter,
//         );
//         assert!(should_run);

//         // Scenario has @smoke and @wip - should not run
//         let should_not_run = runner.should_run_scenario(
//             &["smoke".to_string()],
//             &[],
//             &["wip".to_string()],
//             &[],
//             &filter,
//         );
//         assert!(!should_not_run);
//     }

//     #[test]
//     fn test_examples_tag_filtering_logic() {
//         let runner = Runner::new();

//         // Test tag filtering with Examples tags included
//         let tag_expr = TagOperation::Tag("fast".to_string());
//         let filter = RunnerFilter::with_tags(tag_expr);

//         // Scenario tags without Examples tags - should NOT run
//         let scenario_tags = vec!["slow".to_string()];
//         let examples_tags = vec![];
//         assert!(!runner.should_run_scenario(&[], &[], &scenario_tags, &examples_tags, &filter));

//         // Examples tags with matching filter - should run
//         let scenario_tags = vec!["slow".to_string()];
//         let examples_tags = vec!["fast".to_string()];
//         assert!(runner.should_run_scenario(&[], &[], &scenario_tags, &examples_tags, &filter));

//         // Both scenario and Examples have matching tags - should run
//         let scenario_tags = vec!["fast".to_string()];
//         let examples_tags = vec!["fast".to_string()];
//         assert!(runner.should_run_scenario(&[], &[], &scenario_tags, &examples_tags, &filter));
//     }

//     #[test]
//     fn test_examples_complex_tag_filtering() {
//         let runner = Runner::new();

//         // Test complex expression with Examples: @smoke and not @slow
//         let tag_expr = TagOperation::And(
//             Box::new(TagOperation::Tag("smoke".to_string())),
//             Box::new(TagOperation::Not(Box::new(TagOperation::Tag(
//                 "slow".to_string(),
//             )))),
//         );
//         let filter = RunnerFilter::with_tags(tag_expr);

//         // Examples with @smoke and @fast - should run
//         let scenario_tags = vec![];
//         let examples_tags = vec!["smoke".to_string(), "fast".to_string()];
//         assert!(runner.should_run_scenario(&[], &[], &scenario_tags, &examples_tags, &filter));

//         // Examples with @smoke and @slow - should NOT run
//         let scenario_tags = vec![];
//         let examples_tags = vec!["smoke".to_string(), "slow".to_string()];
//         assert!(!runner.should_run_scenario(&[], &[], &scenario_tags, &examples_tags, &filter));

//         // Feature, scenario, and Examples tags combined
//         let feature_tags = vec!["regression".to_string()];
//         let scenario_tags = vec!["smoke".to_string()];
//         let examples_tags = vec!["fast".to_string()];
//         assert!(runner.should_run_scenario(
//             &feature_tags,
//             &[],
//             &scenario_tags,
//             &examples_tags,
//             &filter
//         ));
//     }
// }
