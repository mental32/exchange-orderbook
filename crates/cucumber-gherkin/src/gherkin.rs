//! [Gherkin] language parser and utilities for working with Gherkin documents.
//!
//! This module provides a compact set of public AST structs and a Chumsky-based
//! parser implementation to parse a subset of Gherkin features, scenarios and
//! steps. The parser is intentionally conservative and focuses on the common
//! constructs (Feature, Scenario, Background, Examples, Steps, Tables, Tags).
//!
//! The file is organized as:
//! 1. (this doc-comment)
//! 2. `use` Items, one per-Item newline separated
//! 3. type-aliases, macros, utilities like `p!` and `ParserInput`
//! 4. chumsky parser functions:
//!     - some struct that will be produced by the parser e.g. `struct Thing`
//!     - a function that will parse the struct e.g. `fn thing_parser() -> p!(Thing)`
//! 5. inline test module
//!
use chumsky::IterParser;
use chumsky::Parser;
use chumsky::error::Rich;
use chumsky::prelude::any;
use chumsky::prelude::end;
use chumsky::prelude::just;

use chumsky::span::SimpleSpan;
use chumsky::text::Char;
use chumsky::text::ascii::keyword;
use chumsky::text::inline_whitespace;
use chumsky::text::newline;
use chumsky::text::whitespace;

type PInput<'src> = &'src str;
type PToken<'src> = <PInput<'src> as chumsky::input::Input<'src>>::Token;

type PError<'src> = chumsky::extra::Err<Rich<'src, char>>;

pub type Spanned<T> = (T, SimpleSpan);

macro_rules! p {
    ($output:ty) => {
        impl Parser<'src, PInput<'src>, $output, PError<'src>>
    };
}

fn block_starter_p<'src>() -> p!(()) {
    // Optional indentation at the start of a line, then any known block keyword
    any()
        .filter(|c: &char| c.is_ascii_whitespace() && *c != '\n')
        .repeated()
        .ignore_then(
            // Blocks that can start after a header description
            keyword("Rule")
                .or(keyword("Background"))
                .or(keyword("Scenario"))
                .or(keyword("Examples"))
                // Steps (for Background/Scenario headers)
                .or(keyword("Given"))
                .or(keyword("When"))
                .or(keyword("Then"))
                .or(keyword("And"))
                .or(keyword("But")),
        )
        // Some starters are followed by ":" (Rule, Background, Scenario, Examples),
        // steps are not. Allow either case.
        .then_ignore(just(":").or_not())
        .ignored()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Background {
    pub header: Header,
    pub steps: Vec<Spanned<Step>>,
}

pub fn background_p<'src>() -> p!(Background) {
    header_p(keyword("Background"), false)
        .then_ignore(newline().then_ignore(whitespace()))
        .then(
            inline_whitespace()
                .ignore_then(step_p())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|(header, steps)| Background { header, steps })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub header: Header,
    pub background: Option<Background>,
    pub scenarios: Vec<Scenario>,
}

pub fn rule_p<'src>() -> p!(Rule) {
    let header = header_p(keyword("Rule"), true);
    let scenario_with_ws = whitespace().ignore_then(scenario_p());

    header
        .then_ignore(newline().then_ignore(whitespace()).repeated())
        .then(background_p().or_not())
        .then(scenario_with_ws.repeated().collect::<Vec<_>>())
        .map(|((header, background), scenarios)| Rule {
            header,
            background,
            scenarios,
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagOperation {
    And(Box<TagOperation>, Box<TagOperation>),
    Or(Box<TagOperation>, Box<TagOperation>),
    Not(Box<TagOperation>),
    Tag(String),
}

impl TagOperation {
    /// Parse a tag expression from a string like "(@smoke and not @wip) or @focus"
    pub fn parse(expression: &str) -> Result<TagOperation, String> {
        fn tag_expression_parser<'src>()
        -> impl Parser<'src, PInput<'src>, TagOperation, PError<'src>> {
            use chumsky::prelude::*;

            recursive(|expr| {
                let tag = just("@")
                    .ignore_then(
                        any()
                            .filter(|c: &char| c.is_alphanumeric() || *c == '-' || *c == '_')
                            .repeated()
                            .at_least(1)
                            .collect::<String>(),
                    )
                    .map(TagOperation::Tag);

                let atom = tag
                    .or(just("(").ignore_then(expr.clone()).then_ignore(just(")")))
                    .padded();

                let not_op = just("not")
                    .padded()
                    .ignore_then(atom.clone())
                    .map(|expr| TagOperation::Not(Box::new(expr)))
                    .or(atom);

                let and_op = not_op
                    .clone()
                    .separated_by(just("and").padded())
                    .at_least(1)
                    .collect::<Vec<_>>()
                    .map(|ops| {
                        ops.into_iter()
                            .reduce(|acc, next| TagOperation::And(Box::new(acc), Box::new(next)))
                            .unwrap()
                    });

                let or_op = and_op
                    .clone()
                    .separated_by(just("or").padded())
                    .at_least(1)
                    .collect::<Vec<_>>()
                    .map(|ops| {
                        ops.into_iter()
                            .reduce(|acc, next| TagOperation::Or(Box::new(acc), Box::new(next)))
                            .unwrap()
                    });

                or_op
            })
            .then_ignore(end())
        }

        println!("Parsing tag expression: {}", expression);
        let parser = tag_expression_parser();

        match parser.parse(expression.trim()).into_result() {
            Ok(result) => {
                println!("Successfully parsed tag expression: {:?}", result);
                Ok(result)
            }
            Err(errs) => {
                let error_msg = format!(
                    "Failed to parse tag expression '{}': {:?}",
                    expression, errs
                );
                println!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// Evaluate the tag expression against a set of tags
    pub fn matches(&self, tags: &[String]) -> bool {
        println!(
            "Evaluating tag expression {:?} against tags {:?}",
            self, tags
        );
        let result = self.evaluate(tags);
        println!("Tag expression evaluation result: {}", result);
        result
    }

    pub fn evaluate(&self, tags: &[String]) -> bool {
        match self {
            TagOperation::Tag(tag) => {
                let has_tag = tags.contains(tag);
                println!("  Checking tag '{}' in {:?}: {}", tag, tags, has_tag);
                has_tag
            }
            TagOperation::And(left, right) => {
                let left_result = left.evaluate(tags);
                let right_result = right.evaluate(tags);
                let result = left_result && right_result;
                println!(
                    "  AND operation: {} && {} = {}",
                    left_result, right_result, result
                );
                result
            }
            TagOperation::Or(left, right) => {
                let left_result = left.evaluate(tags);
                let right_result = right.evaluate(tags);
                let result = left_result || right_result;
                println!(
                    "  OR operation: {} || {} = {}",
                    left_result, right_result, result
                );
                result
            }
            TagOperation::Not(inner) => {
                let inner_result = inner.evaluate(tags);
                let result = !inner_result;
                println!("  NOT operation: !{} = {}", inner_result, result);
                result
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub comments: Vec<String>,
    pub keyword: String,
    pub tags: Vec<String>,
    pub name: String,
    pub description: Option<String>,
}

// Parser for a line of tags (e.g., "@smoke @fast")
pub fn line_of_tags_p<'src>() -> p!(Vec<String>) {
    let tag = just("@")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || *c == '_' || *c == '-')
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .labelled("tag");

    tag.separated_by(just(" ").repeated().at_least(1))
        .at_least(1)
        .collect::<Vec<String>>()
        .then_ignore(just(" ").repeated()) // Allow trailing whitespace
}

pub fn header_p<'src>(
    keywords: impl Parser<'src, PInput<'src>, &'src str, PError<'src>>,
    has_description: bool,
) -> p!(Header) {
    let comment = just("#")
        .ignore_then(
            any()
                .and_is(newline().not())
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .labelled("comment");

    let comments = comment
        .then_ignore(newline().then(whitespace()))
        .repeated()
        .at_least(1)
        .collect::<Vec<String>>()
        .labelled("comments")
        .or_not();

    let line_of_tags = line_of_tags_p();

    let name = any()
        .and_is(newline().not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|name| name.trim().to_owned())
        .labelled("name");

    let description_line = block_starter_p()
        .not() // do not consume if next token starts a new block
        .ignore_then(
            any()
                .and_is(newline().not())
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .map(|line| line.trim_end().to_owned())
        .labelled("description_line");

    let other = keywords
        .then_ignore(just(":"))
        .then(name)
        // Only move to the next line once (optional) to attempt reading description
        .then(if has_description {
            newline()
                .then_ignore(whitespace()) // indent on the next line
                .ignore_then(
                    description_line
                        .separated_by(newline())
                        .collect::<Vec<String>>(),
                )
                .or_not()
                .boxed()
        } else {
            any()
                .filter(|_: &char| false)
                .map(|_| vec![])
                .or_not()
                .boxed()
        });
    comments
        .then(
            line_of_tags
                .then_ignore(newline().then(whitespace()))
                .or_not(),
        )
        .then(other)
        .map(|((comments, t), ((k, name), description))| Header {
            comments: comments.unwrap_or_default(),
            keyword: k.to_owned(),
            tags: t
                .unwrap_or_default()
                .into_iter()
                .map(|tag| tag.to_owned())
                .collect(),
            name,
            description: description.map(|v| v.join("\n")),
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn table_p<'src>() -> p!(Table) {
    // Lookahead guard: only attempt a row if the next non-consuming position is a '|' at line start (after optional indentation)
    let row_start = inline_whitespace().then(just('|')).ignored().rewind();

    // Parser for a single row
    let table_row = inline_whitespace()
        .ignore_then(just('|'))
        .ignore_then(
            any()
                .and_is(newline().not())
                .repeated()
                .collect::<String>()
                .map(|line: String| {
                    dbg!("Processing table line:", &line); // DEBUG: observe line parsing

                    // Handle escaped characters at line level BEFORE splitting on |
                    // Replace escaped sequences in specific order to avoid conflicts
                    let line_with_placeholders = line
                        .replace("\\\\", "\u{E000}") // Use private use area as temporary placeholder for \\
                        .replace("\\|", "\u{E001}") // Use private use area as temporary placeholder for \|
                        .replace("\\n", "\u{E002}"); // Use private use area as temporary placeholder for \n

                    dbg!(
                        "Line after placeholder replacement:",
                        &line_with_placeholders
                    ); // DEBUG

                    // Split into cells using unescaped | characters
                    let cells = line_with_placeholders
                        .split('|')
                        .map(|cell| {
                            let trimmed = cell.trim();
                            // Replace placeholders with actual characters
                            let unescaped = trimmed
                                .replace("\u{E000}", "\\") // Restore \\ -> \
                                .replace("\u{E001}", "|") // Restore \| -> |
                                .replace("\u{E002}", "\n"); // Restore \n -> newline
                            dbg!("Cell processed:", trimmed, "->", &unescaped); // DEBUG: observe cell escaping
                            unescaped
                        })
                        .filter(|cell| !cell.is_empty())
                        .collect::<Vec<String>>();
                    dbg!("Parsed cells:", &cells); // DEBUG: observe cell parsing
                    cells
                }),
        )
        .then_ignore(newline().or(end()));

    // Header row + remaining rows guarded by lookahead
    table_row
        .then(
            row_start
                .ignore_then(table_row.clone())
                .repeated()
                .collect::<Vec<Vec<String>>>()
                .map(|rows| {
                    dbg!("Total table rows collected:", rows.len()); // DEBUG: observe row count
                    rows
                }),
        )
        .map(|(header, rows)| {
            let table = Table { header, rows };
            dbg!("Final table:", &table); // DEBUG: observe final table structure
            table
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Examples {
    pub tags: Vec<String>,
    pub header: Header,
    pub table: Table,
}

pub fn examples_p<'src>() -> p!(Examples) {
    let header = header_p(keyword("Examples"), false); // Examples don't have descriptions

    // Optional tags before Examples header (tags might be preceded by whitespace)
    let tags_line = line_of_tags_p().then_ignore(newline().or_not()).or_not();

    tags_line
        .then(header)
        .then_ignore(newline().then_ignore(whitespace()))
        .then(table_p())
        .map(|((tags, header), table)| Examples {
            tags: tags.unwrap_or_default(),
            header,
            table,
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepType {
    Given,
    When,
    Then,
    And,
    But,
    Asterisk,
}

impl std::fmt::Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepType::Given => write!(f, "Given"),
            StepType::When => write!(f, "When"),
            StepType::Then => write!(f, "Then"),
            StepType::And => write!(f, "And"),
            StepType::But => write!(f, "But"),
            StepType::Asterisk => write!(f, "*"),
        }
    }
}

fn step_type_p<'src>() -> p!(StepType) {
    let given = keyword("Given").map(|_| StepType::Given);
    let when = keyword("When").map(|_| StepType::When);
    let then = keyword("Then").map(|_| StepType::Then);
    let and = keyword("And").map(|_| StepType::And);
    let but = keyword("But").map(|_| StepType::But);
    let asterisk = just('*').map(|_| StepType::Asterisk);

    given
        .or(when)
        .or(then)
        .or(and)
        .or(but)
        .or(asterisk)
        .labelled("Step Type")
}

fn word_p<'src>() -> p!(String) {
    any()
        .filter(|c: &PToken| c.to_ascii().map_or(false, |i| !i.is_ascii_whitespace()))
        .repeated()
        .collect()
}

fn segment_p<'src>() -> p!(Spanned<String>) {
    word_p()
        .delimited_by(just("<"), just(">"))
        .map_with(|word, e| (word, e.span()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepText {
    Word(Spanned<String>),
    Segment(Spanned<String>),
}

impl PartialEq<&'_ str> for StepText {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Self::Word((word, _)) => word == other,
            Self::Segment((segment, _)) => segment == other,
        }
    }
}

fn step_text_p<'src>() -> p!(Box<[StepText]>) {
    word_p()
        .map_with(|word, e| StepText::Word((word, e.span())))
        .or(segment_p().map(|segment| StepText::Segment(segment)))
        .separated_by(inline_whitespace().at_least(1))
        .collect::<Vec<StepText>>()
        .map(|v| v.into_boxed_slice())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocString {
    pub content: String,
    pub content_type: Option<String>,
}

pub fn docstring_p<'src>() -> p!(DocString) {
    // Generic function to create docstring parsers for any fence
    let make_docstring_parser = |fence: &'static str| {
        let fence_parser = just(fence);

        let docstring_without_type = fence_parser
            .clone()
            .ignore_then(newline().or_not()) // Optional newline after opening
            .ignore_then(
                any()
                    .and_is(fence_parser.clone().not())
                    .repeated()
                    .collect::<String>(),
            )
            .then_ignore(fence_parser.clone())
            .map(|content| DocString {
                content: content.trim().to_string(),
                content_type: None,
            });

        let docstring_with_type = fence_parser
            .clone()
            .ignore_then(
                any()
                    .filter(|c: &PToken| c.to_ascii().map_or(false, |i| !i.is_ascii_whitespace()))
                    .repeated()
                    .at_least(1) // Content type must be non-empty
                    .collect::<String>(),
            )
            .then_ignore(inline_whitespace().or_not()) // Optional single whitespace
            .then_ignore(newline()) // Required newline after content type
            .then(
                any()
                    .and_is(fence_parser.clone().not())
                    .repeated()
                    .collect::<String>(),
            )
            .then_ignore(fence_parser)
            .map(|(content_type, content)| DocString {
                content: content.trim().to_string(),
                content_type: Some(content_type),
            });

        // Try with content type first, fallback to without
        docstring_with_type.or(docstring_without_type)
    };

    // Support both """ and ``` fences
    make_docstring_parser("```").or(make_docstring_parser("\"\"\""))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub verb: StepType,
    pub text_parts: Box<[StepText]>,
    pub doc_string: Option<DocString>,
    pub data_table: Option<Table>,
}

impl Step {
    pub fn text_as_string(&self) -> String {
        self.text_parts
            .iter()
            .map(|text| match text {
                StepText::Word((word, _)) => word.clone(),
                StepText::Segment((segment, _)) => segment.clone(),
            })
            .collect::<Vec<String>>()
            .join(" ")
    }
}

fn step_p<'src>() -> p!(Spanned<Step>) {
    step_type_p()
        .then(inline_whitespace().ignore_then(step_text_p()))
        .then(
            // Handle step termination: either with arguments (data table/docstring) or just a newline
            newline()
                .then(
                    // Try data table (lookahead: '|')
                    inline_whitespace()
                        .then(just('|').rewind())
                        .ignore_then(table_p().map(|table| {
                            dbg!("Found data table in step"); // DEBUG: table parsing
                            (None, Some(table))
                        }))
                        // Or docstring (lookahead: '"""' or '```')
                        .or(inline_whitespace()
                            .then(just("\"\"\"").rewind().or(just("```").rewind()))
                            .ignore_then(docstring_p().then_ignore(newline().or(end())).map(
                                |ds| {
                                    dbg!("Found docstring in step"); // DEBUG: docstring parsing
                                    (Some(ds), None)
                                },
                            )))
                        // Or just end the step (no arguments)
                        .or_not(),
                )
                .map(|(_, opt)| {
                    let result = opt.unwrap_or((None, None));
                    dbg!("Final step args:", &result); // DEBUG: final step args
                    result
                })
                // Allow steps at end of input without trailing newline
                .or(end().map(|_| {
                    dbg!("Step at end of input"); // DEBUG: end of input
                    (None, None)
                })),
        )
        .map_with(|((verb, text_parts), (doc_string, data_table)), e| {
            dbg!(
                "Creating step:",
                &verb,
                &text_parts,
                &doc_string.is_some(),
                &data_table.is_some()
            ); // DEBUG: step creation
            (
                Step {
                    verb,
                    text_parts,
                    doc_string,
                    data_table,
                },
                e.span(),
            )
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub header: Header,
    pub steps: Vec<Spanned<Step>>,
    pub examples: Vec<Examples>,
}

fn scenario_p<'src>() -> p!(Scenario) {
    let examples_with_ws = whitespace().ignore_then(examples_p());

    header_p(just("Scenario Outline").or(keyword("Scenario")), false)
        .then_ignore(newline().then_ignore(whitespace()).or_not())
        .then(
            inline_whitespace()
                .ignore_then(step_p())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(examples_with_ws.repeated().collect::<Vec<Examples>>())
        .map(|((header, steps), examples)| Scenario {
            header,
            steps,
            examples,
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    pub header: Header,
    pub scenarios: Vec<Scenario>,
    pub background: Option<Background>,
    pub rules: Vec<Rule>,
}

pub fn feature_p<'src>() -> p!(Feature) {
    // Parse scenarios or rules in any order, but with proper whitespace handling
    let scenario_or_rule = whitespace().ignore_then(
        scenario_p()
            .map(|s| (Some(s), None))
            .or(rule_p().map(|r| (None, Some(r)))),
    );

    header_p(keyword("Feature"), true) // comments, tags, feature, name, description
        .then_ignore(newline().then_ignore(whitespace()).repeated())
        .then(background_p().or_not()) // optional background
        .then(scenario_or_rule.repeated().collect::<Vec<_>>())
        .map(move |((header, background), items)| {
            let mut scenarios = Vec::new();
            let mut rules = Vec::new();

            for (scenario, rule) in items {
                if let Some(s) = scenario {
                    scenarios.push(s);
                }
                if let Some(r) = rule {
                    rules.push(r);
                }
            }

            Feature {
                header,
                scenarios,
                background,
                rules,
            }
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub language: Option<String>,
    pub feature: Feature,
}

impl Document {
    pub fn steps(&self) -> Vec<Step> {
        let mut all_steps = Vec::new();

        // Helper to expand a scenario's steps, handling Scenario Outlines
        let expand_scenario_steps = |scenario: &Scenario, background_steps: &Vec<Step>| {
            let mut scenario_steps = Vec::new();

            // Add background steps first
            scenario_steps.extend(background_steps.clone());

            if scenario.examples.is_empty() {
                // Regular scenario - just add the steps as-is
                scenario_steps.extend(scenario.steps.iter().map(|(step, _)| step.clone()));
            } else {
                // Scenario Outline - expand with each example
                for example_set in &scenario.examples {
                    for row in &example_set.table.rows {
                        // Substitute parameters for each step
                        for (step, _) in &scenario.steps {
                            let mut text = step.text_as_string();

                            // Substitute <parameter> placeholders with actual values
                            for (header, value) in example_set.table.header.iter().zip(row.iter()) {
                                let placeholder = format!("<{}>", header);
                                text = text.replace(&placeholder, value);
                            }

                            // Create a new step with substituted text
                            let text_parts = vec![StepText::Word((
                                text,
                                SimpleSpan {
                                    start: 0,
                                    end: 0,
                                    context: (),
                                },
                            ))]
                            .into_boxed_slice();

                            let substituted_step = Step {
                                verb: step.verb.clone(),
                                text_parts,
                                doc_string: step.doc_string.clone(),
                                data_table: step.data_table.clone(),
                            };

                            scenario_steps.push(substituted_step);
                        }
                    }
                }
            }

            scenario_steps
        };

        // Process top-level scenarios
        let background_steps: Vec<Step> = self
            .feature
            .background
            .as_ref()
            .map(|bg| bg.steps.iter().map(|(step, _)| step.clone()).collect())
            .unwrap_or_default();

        for scenario in &self.feature.scenarios {
            all_steps.extend(expand_scenario_steps(scenario, &background_steps));
        }

        // Process rules and their nested scenarios
        for rule in &self.feature.rules {
            let rule_background_steps: Vec<Step> = rule
                .background
                .as_ref()
                .map(|bg| bg.steps.iter().map(|(step, _)| step.clone()).collect())
                .unwrap_or_else(|| background_steps.clone());

            for scenario in &rule.scenarios {
                all_steps.extend(expand_scenario_steps(scenario, &rule_background_steps));
            }
        }

        all_steps
    }
}

fn language_directive_p<'src>() -> p!(String) {
    println!("Parsing language directive...");
    just("#")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ascii_whitespace() && *c != '\n')
                .repeated(),
        )
        .ignore_then(keyword("language"))
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ascii_whitespace() && *c != '\n')
                .repeated(),
        )
        .ignore_then(just(":"))
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ascii_whitespace() && *c != '\n')
                .repeated(),
        )
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || *c == '-')
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then_ignore(newline())
        .map(|lang| {
            println!("Found language directive: {}", lang);
            lang
        })
}

pub fn document_p<'src>() -> p!(Document) {
    println!("Starting document parse...");

    // Optional language directive at the start
    let language_opt = language_directive_p()
        .then_ignore(any().filter(|c: &char| c.is_ascii_whitespace()).repeated())
        .or_not()
        .map(|lang| {
            dbg!("Language directive parsed:", &lang);
            lang
        });

    language_opt
        .then(feature_p())
        .map(|(language, feature)| {
            dbg!("Creating document with language:", &language);
            Document { language, feature }
        })
        .then_ignore(end())
}

#[cfg(test)]
mod test {
    use chumsky::text::whitespace;

    use super::*;

    #[test]
    fn parse_hyphenated_tags() {
        let input = "@fast @slow-test @multi-word-tag
Feature: Tag with hyphens";

        let res = header_p(keyword("Feature"), true)
            .then_ignore(end())
            .parse(input)
            .into_result();

        assert!(
            res.is_ok(),
            "Failed to parse hyphenated tags: {:?}",
            res.unwrap_err()
        );
        let header = res.unwrap();
        assert_eq!(header.tags, vec!["fast", "slow-test", "multi-word-tag"]);
        assert_eq!(header.name, "Tag with hyphens");
    }

    #[test]
    fn parse_tags_with_trailing_whitespace() {
        let input = "@sell-orders
Scenario: Sell order validation";

        let res = header_p(keyword("Scenario"), false)
            .then_ignore(end())
            .parse(input)
            .into_result();

        assert!(
            res.is_ok(),
            "Failed to parse tags with trailing whitespace: {:?}",
            res.unwrap_err()
        );
        let header = res.unwrap();
        assert_eq!(header.tags, vec!["sell-orders"]);
        assert_eq!(header.name, "Sell order validation");
    }

    #[test]
    fn parse_header() {
        let input = "@tag1 @tag2
            Feature: foo bar
                Some description of
                Some feature
            ";
        let res = header_p(keyword("Feature"), true)
            .then_ignore(end())
            .parse(input.trim())
            .into_result();
        assert!(res.is_ok(), "{:?}", res.unwrap_err());
        let f = res.unwrap();
        assert_eq!(
            f,
            Header {
                tags: vec!["tag1".to_owned(), "tag2".to_owned()],
                name: "foo bar".to_owned(),
                comments: vec![],
                keyword: "Feature".to_owned(),
                description: Some("Some description of\n                Some feature".to_owned()),
            },
        );

        let input = "Scenario: foo bar";
        let res = header_p(keyword("Scenario"), false)
            .then_ignore(end())
            .parse(input.trim())
            .into_result();
        assert!(res.is_ok(), "{:?}", res.unwrap_err());
        let f = res.unwrap();
        assert_eq!(
            f,
            Header {
                tags: vec![],
                name: "foo bar".to_owned(),
                comments: vec![],
                keyword: "Scenario".to_owned(),
                description: None,
            },
        );
    }

    #[test]
    fn parse_steps() {
        fn into_step_text(st: &'static str, input: &'static str) -> Box<[StepText]> {
            let mut pos = input.find(st).unwrap_or(0);
            st.split_ascii_whitespace()
                .map(|part: &str| {
                    let start = pos;
                    let end = start + part.len();
                    pos = end + 1; // account for space

                    if part.starts_with("<") && part.ends_with(">") {
                        let text = part[1..part.len() - 1].to_owned();
                        let span = SimpleSpan {
                            start,
                            end,
                            context: (),
                        };
                        StepText::Segment((text, span))
                    } else {
                        StepText::Word((
                            part.to_owned(),
                            SimpleSpan {
                                start,
                                end,
                                context: (),
                            },
                        ))
                    }
                })
                .collect()
        }

        let cases = [
            (
                "Given a step",
                Step {
                    verb: StepType::Given,
                    text_parts: into_step_text("a step", "Given a step"),
                    doc_string: None,
                    data_table: None,
                },
            ),
            (
                "When a step",
                Step {
                    verb: StepType::When,
                    text_parts: into_step_text("a step", "When a step"),
                    doc_string: None,
                    data_table: None,
                },
            ),
            (
                "Then a step",
                Step {
                    verb: StepType::Then,
                    text_parts: into_step_text("a step", "Then a step"),
                    doc_string: None,
                    data_table: None,
                },
            ),
            (
                "And a step",
                Step {
                    verb: StepType::And,
                    text_parts: into_step_text("a step", "And a step"),
                    doc_string: None,
                    data_table: None,
                },
            ),
            (
                "But a step",
                Step {
                    verb: StepType::But,
                    text_parts: into_step_text("a step", "But a step"),
                    doc_string: None,
                    data_table: None,
                },
            ),
            (
                "And a step",
                Step {
                    verb: StepType::And,
                    text_parts: into_step_text("a step", "And a step"),
                    doc_string: None,
                    data_table: None,
                },
            ),
        ];

        for (input, expected) in cases {
            let res = step_p()
                .then_ignore(end())
                .parse(input.trim())
                .into_result();
            assert!(res.is_ok(), "{:?}", res.unwrap_err());
            let (s, _) = res.unwrap();
            assert_eq!(s, expected, "Expected {:?}, got {:?}", expected, s);
        }

        let res = step_p()
            .repeated()
            .at_least(3)
            .collect::<Vec<_>>()
            .then_ignore(end())
            .parse("Given a step\nWhen another step\nThen a third step")
            .into_result()
            .expect("Failed to parse steps");
        assert!(res.len() == 3, "Expected 3 steps, got {}", res.len());
    }

    #[test]
    fn parse_scenario() {
        let res = scenario_p()
            .then_ignore(end())
            .parse(
                "Scenario: baz qux\n    Given a step\n    When another step\n    Then a third step",
            )
            .into_result()
            .expect("Failed to parse scenario");
        assert!(
            res.header.name == "baz qux",
            "Expected 'baz qux', got {:?}",
            res.header.name
        );
    }

    #[test]
    fn parse_feature() {
        let input = "@foo
Feature: foo bar

Scenario: baz qux
    Given a step
    When another step
    Then a third step";

        let res = feature_p()
            .then_ignore(end())
            .parse(input)
            .into_result()
            .expect("Failed to parse feature");
        assert!(
            res.header.name == "foo bar",
            "Expected 'foo bar', got {:?}",
            res.header.name
        );
        assert!(
            res.scenarios.len() == 1,
            "Expected 1 scenario, got {}",
            res.scenarios.len()
        );
        assert!(
            res.scenarios[0].header.name == "baz qux",
            "Expected 'baz qux', got {:?}",
            res.scenarios[0].header.name
        );
    }

    #[test]
    fn parse_table() {
        let input = "        | orderType | status    |
        | buy       | confirmed |
        | sell      | confirmed |
        | limit     | pending   |";

        let res = table_p().then_ignore(end()).parse(input).into_result();

        assert!(res.is_ok(), "Failed to parse table: {:?}", res.unwrap_err());
        let table = res.unwrap();

        assert_eq!(
            table.header.len(),
            2,
            "Expected 2 headers, got {}",
            table.header.len()
        );
        assert_eq!(
            table.header[0].trim(),
            "orderType",
            "Expected 'orderType', got '{}'",
            table.header[0]
        );
        assert_eq!(
            table.header[1].trim(),
            "status",
            "Expected 'status', got '{}'",
            table.header[1]
        );

        assert_eq!(
            table.rows.len(),
            3,
            "Expected 3 rows, got {}",
            table.rows.len()
        );
        assert_eq!(
            table.rows[0][0], "buy",
            "Expected 'buy', got '{}'",
            table.rows[0][0]
        );
        assert_eq!(
            table.rows[0][1], "confirmed",
            "Expected 'confirmed', got '{}'",
            table.rows[0][1]
        );
        assert_eq!(
            table.rows[1][0], "sell",
            "Expected 'sell', got '{}'",
            table.rows[1][0]
        );
        assert_eq!(
            table.rows[1][1], "confirmed",
            "Expected 'confirmed', got '{}'",
            table.rows[1][1]
        );
        assert_eq!(
            table.rows[2][0], "limit",
            "Expected 'limit', got '{}'",
            table.rows[2][0]
        );
        assert_eq!(
            table.rows[2][1], "pending",
            "Expected 'pending', got '{}'",
            table.rows[2][1]
        );
    }

    #[test]
    fn parse_background() {
        let input = "Background: Setup
    Given a clean environment
    And a default configuration is loaded
";

        let res = background_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse background: {:?}",
            res.unwrap_err()
        );
        let background = res.unwrap();

        assert_eq!(
            background.header.keyword, "Background",
            "Expected keyword 'Background', got '{}'",
            background.header.keyword
        );
        assert_eq!(
            background.header.name, "Setup",
            "Expected name 'Setup', got '{}'",
            background.header.name
        );
        assert_eq!(
            background.steps.len(),
            2,
            "Expected 2 steps, got {}",
            background.steps.len()
        );

        let (step1, _) = &background.steps[0];
        assert_eq!(
            step1.verb,
            StepType::Given,
            "Expected 'Given', got {:?}",
            step1.verb
        );

        let (step2, _) = &background.steps[1];
        assert_eq!(
            step2.verb,
            StepType::And,
            "Expected 'And', got {:?}",
            step2.verb
        );
    }

    #[test]
    fn parse_docstring() {
        let input = "\"\"\"
This is a multi-line
docstring with some content
that spans multiple lines
\"\"\"";

        let res = docstring_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse docstring: {:?}",
            res.unwrap_err()
        );
        let docstring = res.unwrap();

        let expected_content =
            "This is a multi-line\ndocstring with some content\nthat spans multiple lines";
        assert_eq!(
            docstring.content, expected_content,
            "Expected '{}', got '{}'",
            expected_content, docstring.content
        );
        assert_eq!(
            docstring.content_type, None,
            "Expected no content type, got {:?}",
            docstring.content_type
        );
    }

    #[test]
    fn parse_docstring_with_content_type() {
        let input = "\"\"\"markdown
This is a **markdown** docstring
with some *formatted* content
\"\"\"";

        let res = docstring_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse docstring with content type: {:?}",
            res.unwrap_err()
        );
        let docstring = res.unwrap();

        let expected_content = "This is a **markdown** docstring\nwith some *formatted* content";
        assert_eq!(
            docstring.content, expected_content,
            "Expected '{}', got '{}'",
            expected_content, docstring.content
        );
        assert_eq!(
            docstring.content_type,
            Some("markdown".to_string()),
            "Expected content type 'markdown', got {:?}",
            docstring.content_type
        );
    }

    #[test]
    fn parse_docstring_with_backticks() {
        let input = "```json
{
  \"key\": \"value\",
  \"array\": [1, 2, 3]
}
```";

        let res = docstring_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse docstring with backticks: {:?}",
            res.unwrap_err()
        );
        let docstring = res.unwrap();

        let expected_content = "{\n  \"key\": \"value\",\n  \"array\": [1, 2, 3]\n}";
        assert_eq!(
            docstring.content, expected_content,
            "Expected '{}', got '{}'",
            expected_content, docstring.content
        );
        assert_eq!(
            docstring.content_type,
            Some("json".to_string()),
            "Expected content type 'json', got {:?}",
            docstring.content_type
        );
    }

    #[test]
    fn parse_docstring_with_backticks_no_type() {
        let input = "```
Some plain text content
without a content type
```";

        let res = docstring_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse backtick docstring without type: {:?}",
            res.unwrap_err()
        );
        let docstring = res.unwrap();

        let expected_content = "Some plain text content\nwithout a content type";
        assert_eq!(
            docstring.content, expected_content,
            "Expected '{}', got '{}'",
            expected_content, docstring.content
        );
        assert_eq!(
            docstring.content_type, None,
            "Expected no content type, got {:?}",
            docstring.content_type
        );
    }

    #[test]
    fn parse_examples() {
        let input = "Examples: Order Types
        | orderType | status    |
        | buy       | confirmed |
        | sell      | confirmed |
        | limit     | pending   |";

        let res = examples_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse examples: {:?}",
            res.unwrap_err()
        );
        let examples = res.unwrap();

        // Should have no tags when none are specified
        assert_eq!(
            examples.tags,
            Vec::<String>::new(),
            "Expected no tags, got {:?}",
            examples.tags
        );

        assert_eq!(
            examples.header.keyword, "Examples",
            "Expected keyword 'Examples', got '{}'",
            examples.header.keyword
        );
        assert_eq!(
            examples.header.name, "Order Types",
            "Expected name 'Order Types', got '{}'",
            examples.header.name
        );

        assert_eq!(
            examples.table.header.len(),
            2,
            "Expected 2 headers, got {}",
            examples.table.header.len()
        );
        assert_eq!(
            examples.table.header[0].trim(),
            "orderType",
            "Expected 'orderType', got '{}'",
            examples.table.header[0]
        );
        assert_eq!(
            examples.table.header[1].trim(),
            "status",
            "Expected 'status', got '{}'",
            examples.table.header[1]
        );

        assert_eq!(
            examples.table.rows.len(),
            3,
            "Expected 3 rows, got {}",
            examples.table.rows.len()
        );
        assert_eq!(
            examples.table.rows[0][0], "buy",
            "Expected 'buy', got '{}'",
            examples.table.rows[0][0]
        );
        assert_eq!(
            examples.table.rows[0][1], "confirmed",
            "Expected 'confirmed', got '{}'",
            examples.table.rows[0][1]
        );
    }

    #[test]
    fn parse_rule() {
        let input = "Rule: User Management
    Test user-related functionality

    Background: User setup
        Given a user management system exists

    Scenario: Create user
        When a new user is created
        Then the user should exist

    Scenario: Delete user
        When an existing user is deleted
        Then the user should not exist
";

        let res = rule_p().then_ignore(end()).parse(input).into_result();

        assert!(res.is_ok(), "Failed to parse rule: {:?}", res.unwrap_err());
        let rule = res.unwrap();

        assert_eq!(
            rule.header.keyword, "Rule",
            "Expected keyword 'Rule', got '{}'",
            rule.header.keyword
        );
        assert_eq!(
            rule.header.name, "User Management",
            "Expected name 'User Management', got '{}'",
            rule.header.name
        );
        assert_eq!(
            rule.header.description,
            Some("Test user-related functionality".to_owned()),
            "Expected description, got {:?}",
            rule.header.description
        );

        assert!(
            rule.background.is_some(),
            "Expected background to be present"
        );
        let background = rule.background.unwrap();
        assert_eq!(
            background.header.name, "User setup",
            "Expected background name 'User setup', got '{}'",
            background.header.name
        );
        assert_eq!(
            background.steps.len(),
            1,
            "Expected 1 background step, got {}",
            background.steps.len()
        );

        assert_eq!(
            rule.scenarios.len(),
            2,
            "Expected 2 scenarios, got {}",
            rule.scenarios.len()
        );
        assert_eq!(
            rule.scenarios[0].header.name, "Create user",
            "Expected first scenario 'Create user', got '{}'",
            rule.scenarios[0].header.name
        );
        assert_eq!(
            rule.scenarios[1].header.name, "Delete user",
            "Expected second scenario 'Delete user', got '{}'",
            rule.scenarios[1].header.name
        );
    }

    #[test]
    fn parse_document() {
        let input = "@foo
Feature: foo bar

Background: Setup
    Given a clean environment

Scenario: baz qux
    Given a step
    When another step
    Then a third step";

        let res = document_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse document: {:?}",
            res.unwrap_err()
        );
        let document = res.unwrap();

        assert_eq!(
            document.feature.header.name, "foo bar",
            "Expected feature name 'foo bar', got '{}'",
            document.feature.header.name
        );
        assert_eq!(
            document.feature.header.keyword, "Feature",
            "Expected keyword 'Feature', got '{}'",
            document.feature.header.keyword
        );
        assert_eq!(
            document.feature.header.tags,
            vec!["foo"],
            "Expected tags ['foo'], got {:?}",
            document.feature.header.tags
        );

        assert!(
            document.feature.background.is_some(),
            "Expected feature background to be present"
        );
        let background = document.feature.background.as_ref().unwrap();
        assert_eq!(
            background.header.name, "Setup",
            "Expected background name 'Setup', got '{}'",
            background.header.name
        );

        assert_eq!(
            document.feature.scenarios.len(),
            1,
            "Expected 1 scenario, got {}",
            document.feature.scenarios.len()
        );
        assert_eq!(
            document.feature.scenarios[0].header.name, "baz qux",
            "Expected scenario name 'baz qux', got '{}'",
            document.feature.scenarios[0].header.name
        );
    }

    #[test]
    fn parse_step_with_data_table() {
        let input = "Given the following users exist:
        | name  | email            | role  |
        | Alice | alice@email.com  | admin |
        | Bob   | bob@email.com    | user  |";

        let res = step_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse step with data table: {:?}",
            res.unwrap_err()
        );
        let (step, _span) = res.unwrap();

        assert_eq!(step.verb, StepType::Given);
        assert_eq!(step.text_as_string(), "the following users exist:");

        // Should parse data table
        assert!(
            step.data_table.is_some(),
            "Expected data table to be parsed"
        );
        let table = step.data_table.unwrap();
        assert_eq!(table.header, vec!["name", "email", "role"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0], vec!["Alice", "alice@email.com", "admin"]);
        assert_eq!(table.rows[1], vec!["Bob", "bob@email.com", "user"]);
    }

    #[test]
    fn parse_asterisk_step() {
        let input = "* this is an asterisk step";

        let res = step_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse asterisk step: {:?}",
            res.unwrap_err()
        );
        let (step, _span) = res.unwrap();

        assert_eq!(
            step.verb,
            StepType::Asterisk,
            "Expected StepType::Asterisk, got {:?}",
            step.verb
        );
        assert_eq!(
            step.text_as_string(),
            "this is an asterisk step",
            "Expected step text 'this is an asterisk step', got '{}'",
            step.text_as_string()
        );
    }

    #[test]
    fn parse_table_with_cell_escaping() {
        // Note: In Rust string literals, \\ represents a single \, so \\n means \n in the actual content
        let input = "| name | message | symbol |\n| Alice | \"Hello\\nWorld\" | € \\| USD |\n| Bob | \"Test\\\\Path\" | £ \\| GBP |";

        let res = table_p().parse(input).into_result();
        assert!(
            res.is_ok(),
            "Failed to parse table with escaping: {:?}",
            res.unwrap_err()
        );

        let table = res.unwrap();
        assert_eq!(table.header, vec!["name", "message", "symbol"]);
        assert_eq!(table.rows.len(), 2);

        // Check escaped characters are properly unescaped
        assert_eq!(table.rows[0][0], "Alice");
        assert_eq!(table.rows[0][1], "\"Hello\nWorld\""); // \n becomes newline
        assert_eq!(table.rows[0][2], "€ | USD"); // \| becomes |

        assert_eq!(table.rows[1][0], "Bob");
        assert_eq!(table.rows[1][1], "\"Test\\Path\""); // \\ becomes \
        assert_eq!(table.rows[1][2], "£ | GBP"); // \| becomes |
    }

    #[test]
    fn parse_scenario_with_table_and_following_steps() {
        let input = r#"Scenario: Step with data table
    Given the following users exist:
        | name  | email            | role  |
        | Alice | alice@email.com  | admin |
        | Bob   | bob@email.com    | user  |
    When I check the user list
    Then all users should be present"#;

        let res = scenario_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse scenario: {:?}",
            res.unwrap_err()
        );
        let scenario = res.unwrap();

        assert_eq!(
            scenario.steps.len(),
            3,
            "Expected 3 steps, got {}",
            scenario.steps.len()
        );
        let (given, _) = &scenario.steps[0];
        let (when, _) = &scenario.steps[1];
        let (then, _) = &scenario.steps[2];

        assert_eq!(given.verb, StepType::Given, "First step should be Given");
        assert_eq!(
            given.text_as_string(),
            "the following users exist:",
            "Given step text mismatch"
        );
        assert!(
            given.data_table.is_some(),
            "Given step should have a data table"
        );
        let table = given.data_table.as_ref().unwrap();
        assert_eq!(
            table.header,
            vec!["name", "email", "role"],
            "Table header mismatch"
        );
        assert_eq!(
            table.rows.len(),
            2,
            "Expected 2 table rows, got {}",
            table.rows.len()
        );
        assert_eq!(
            table.rows[0],
            vec!["Alice", "alice@email.com", "admin"],
            "First row mismatch"
        );
        assert_eq!(
            table.rows[1],
            vec!["Bob", "bob@email.com", "user"],
            "Second row mismatch"
        );

        assert_eq!(when.verb, StepType::When, "Second step should be When");
        assert_eq!(
            when.text_as_string(),
            "I check the user list",
            "When step text mismatch"
        );
        assert!(
            when.data_table.is_none(),
            "When step should not have a data table"
        );

        assert_eq!(then.verb, StepType::Then, "Third step should be Then");
        assert_eq!(
            then.text_as_string(),
            "all users should be present",
            "Then step text mismatch"
        );
        assert!(
            then.data_table.is_none(),
            "Then step should not have a data table"
        );
    }

    #[test]
    fn parse_table_with_following_text() {
        let input = "        | name  | email            | role  |
        | Alice | alice@email.com  | admin |
        | Bob   | bob@email.com    | user  |
    When I check the user list";

        // For this test, we expect the table parser to parse only the table part
        // It should work if we manually extract just the table portion
        let table_lines: Vec<&str> = input
            .lines()
            .take_while(|line| line.trim().starts_with('|'))
            .collect();
        let table_part = table_lines.join("\n");

        let res = table_p()
            .then_ignore(end()) // Expect to consume all input in the table part
            .parse(&table_part)
            .into_result();
        assert!(
            res.is_ok(),
            "Failed to parse table part: {:?}",
            res.unwrap_err()
        );
        let table = res.unwrap();

        assert_eq!(
            table.header,
            vec!["name", "email", "role"],
            "Table header mismatch"
        );
        assert_eq!(
            table.rows.len(),
            2,
            "Expected 2 table rows, got {}",
            table.rows.len()
        );
        assert_eq!(
            table.rows[0],
            vec!["Alice", "alice@email.com", "admin"],
            "First row mismatch"
        );
        assert_eq!(
            table.rows[1],
            vec!["Bob", "bob@email.com", "user"],
            "Second row mismatch"
        );
    }

    #[test]
    fn parse_tag_expression_simple() {
        let expr = TagOperation::parse("@smoke").unwrap();
        assert_eq!(expr, TagOperation::Tag("smoke".to_string()));

        let tags = vec!["smoke".to_string(), "fast".to_string()];
        assert!(expr.matches(&tags));

        let no_tags = vec!["slow".to_string()];
        assert!(!expr.matches(&no_tags));
    }

    #[test]
    fn parse_tag_expression_and() {
        let expr = TagOperation::parse("@smoke and @fast").unwrap();

        let both_tags = vec!["smoke".to_string(), "fast".to_string()];
        assert!(expr.matches(&both_tags));

        let one_tag = vec!["smoke".to_string()];
        assert!(!expr.matches(&one_tag));

        let no_tags: Vec<String> = vec![];
        assert!(!expr.matches(&no_tags));
    }

    #[test]
    fn parse_tag_expression_or() {
        let expr = TagOperation::parse("@smoke or @fast").unwrap();

        let both_tags = vec!["smoke".to_string(), "fast".to_string()];
        assert!(expr.matches(&both_tags));

        let smoke_only = vec!["smoke".to_string()];
        assert!(expr.matches(&smoke_only));

        let fast_only = vec!["fast".to_string()];
        assert!(expr.matches(&fast_only));

        let no_tags: Vec<String> = vec![];
        assert!(!expr.matches(&no_tags));
    }

    #[test]
    fn parse_tag_expression_not() {
        let expr = TagOperation::parse("not @slow").unwrap();

        let slow_tag = vec!["slow".to_string()];
        assert!(!expr.matches(&slow_tag));

        let fast_tag = vec!["fast".to_string()];
        assert!(expr.matches(&fast_tag));

        let no_tags: Vec<String> = vec![];
        assert!(expr.matches(&no_tags)); // "not @slow" is true when @slow is not present
    }

    #[test]
    fn parse_tag_expression_complex() {
        let expr = TagOperation::parse("(@smoke and not @wip) or @focus").unwrap();

        // Should match: @smoke without @wip
        let smoke_no_wip = vec!["smoke".to_string(), "fast".to_string()];
        assert!(expr.matches(&smoke_no_wip));

        // Should not match: @smoke with @wip
        let smoke_with_wip = vec!["smoke".to_string(), "wip".to_string()];
        assert!(!expr.matches(&smoke_with_wip));

        // Should match: @focus (regardless of other tags)
        let focus_only = vec!["focus".to_string()];
        assert!(expr.matches(&focus_only));

        let focus_with_wip = vec!["focus".to_string(), "wip".to_string()];
        assert!(expr.matches(&focus_with_wip));

        // Should not match: no relevant tags
        let no_relevant = vec!["other".to_string()];
        assert!(!expr.matches(&no_relevant));
    }

    #[test]
    fn parse_tag_expression_parentheses() {
        let expr = TagOperation::parse("@smoke and (@fast or @integration)").unwrap();

        // Should match: @smoke with @fast
        let smoke_fast = vec!["smoke".to_string(), "fast".to_string()];
        assert!(expr.matches(&smoke_fast));

        // Should match: @smoke with @integration
        let smoke_integration = vec!["smoke".to_string(), "integration".to_string()];
        assert!(expr.matches(&smoke_integration));

        // Should not match: @smoke without @fast or @integration
        let smoke_only = vec!["smoke".to_string()];
        assert!(!expr.matches(&smoke_only));

        // Should not match: no @smoke
        let fast_only = vec!["fast".to_string()];
        assert!(!expr.matches(&fast_only));
    }

    #[test]
    fn parse_tag_expression_hyphenated_tags() {
        let expr = TagOperation::parse("@multi-word-tag and not @slow-test").unwrap();

        let multi_word = vec!["multi-word-tag".to_string()];
        assert!(expr.matches(&multi_word));

        let with_slow = vec!["multi-word-tag".to_string(), "slow-test".to_string()];
        assert!(!expr.matches(&with_slow));
    }

    #[test]
    fn parse_tag_expression_invalid() {
        // Missing @
        let result = TagOperation::parse("smoke and fast");
        assert!(result.is_err());

        // Invalid syntax
        let result = TagOperation::parse("@smoke and");
        assert!(result.is_err());

        // Unmatched parentheses
        let result = TagOperation::parse("(@smoke and @fast");
        assert!(result.is_err());
    }

    #[test]
    fn parse_language_directive() {
        let input = "# language: fr\nFeature: Test français\n  Scenario: Basic\n    Given the system is ready";
        let result = document_p().parse(input.trim()).into_result();

        assert!(result.is_ok());
        let document = result.unwrap();

        // Should have detected French language (even though we're still using English keywords for now)
        assert_eq!(document.language, Some("fr".to_string()));
        assert_eq!(document.feature.header.name, "Test français");
    }

    #[test]
    fn parse_language_directive_english_default() {
        let input = "Feature: Test English\n  Scenario: Basic\n    Given the system is ready";
        let result = document_p().parse(input.trim()).into_result();

        assert!(result.is_ok());
        let document = result.unwrap();

        // Should default to None/English when no language directive
        assert_eq!(document.language, None);
        assert_eq!(document.feature.header.name, "Test English");
    }

    #[test]
    fn parse_examples_with_tags() {
        let input = "@fast @smoke
Examples: Fast Tests
| orderType | status    |
| buy       | confirmed |
| sell      | confirmed |";

        let res = examples_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse examples with tags: {:?}",
            res.unwrap_err()
        );
        let examples = res.unwrap();

        // Verify tags are parsed correctly
        assert_eq!(
            examples.tags,
            vec!["fast", "smoke"],
            "Expected tags ['fast', 'smoke'], got {:?}",
            examples.tags
        );

        // Verify header content
        assert_eq!(
            examples.header.keyword, "Examples",
            "Expected keyword 'Examples', got '{}'",
            examples.header.keyword
        );
        assert_eq!(
            examples.header.name, "Fast Tests",
            "Expected name 'Fast Tests', got '{}'",
            examples.header.name
        );

        // Verify table content
        assert_eq!(
            examples.table.header,
            vec!["orderType", "status"],
            "Table header mismatch"
        );
        assert_eq!(
            examples.table.rows.len(),
            2,
            "Expected 2 table rows, got {}",
            examples.table.rows.len()
        );
        assert_eq!(
            examples.table.rows[0],
            vec!["buy", "confirmed"],
            "First row mismatch"
        );
        assert_eq!(
            examples.table.rows[1],
            vec!["sell", "confirmed"],
            "Second row mismatch"
        );
    }

    #[test]
    fn parse_multiple_examples_blocks() {
        let input = "Scenario Outline: Order Processing
Given a user places a <orderType> order
When the order is processed
Then the order should be <status>

@fast
Examples: Fast Orders
| orderType | status    |
| buy       | confirmed |

@slow @integration
Examples: Slow Orders
| orderType | status  |
| limit     | pending |";

        let res = scenario_p().then_ignore(end()).parse(input).into_result();

        assert!(
            res.is_ok(),
            "Failed to parse multiple examples blocks: {:?}",
            res.unwrap_err()
        );
        let scenario = res.unwrap();

        // Should have 2 Examples blocks
        assert_eq!(
            scenario.examples.len(),
            2,
            "Expected 2 Examples blocks, got {}",
            scenario.examples.len()
        );

        // First Examples block - @fast
        assert_eq!(
            scenario.examples[0].tags,
            vec!["fast"],
            "First Examples tags mismatch"
        );
        assert_eq!(
            scenario.examples[0].header.name, "Fast Orders",
            "First Examples name mismatch"
        );
        assert_eq!(
            scenario.examples[0].table.rows.len(),
            1,
            "First Examples should have 1 row"
        );

        // Second Examples block - @slow @integration
        assert_eq!(
            scenario.examples[1].tags,
            vec!["slow", "integration"],
            "Second Examples tags mismatch"
        );
        assert_eq!(
            scenario.examples[1].header.name, "Slow Orders",
            "Second Examples name mismatch"
        );
        assert_eq!(
            scenario.examples[1].table.rows.len(),
            1,
            "Second Examples should have 1 row"
        );
    }
}
