use ariadne::Color;
use ariadne::Label;
use ariadne::Report;
use ariadne::ReportKind;
use ariadne::sources;
use chumsky::Parser;
use clap::Parser as _;
use cucumber_gherkin::gherkin::TagOperation;
use cucumber_gherkin::gherkin::document_p;
use cucumber_gherkin::runner::Runner;
use cucumber_gherkin::runner::RunnerFilter;
use cucumber_gherkin::steps::DefaultSteps;
use cucumber_gherkin::steps::Steps;
use std::path::PathBuf;

#[derive(Debug, Clone, clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Files to parse and run tests on.
    #[clap(value_parser)]
    files: Vec<PathBuf>,
    /// dry run will not execute the steps only parse and validate the gherkin
    #[clap(long, default_value_t = false)]
    dry_run: bool,
    /// Tag expression to filter scenarios (e.g., "@smoke", "@smoke and not @wip", "(@fast or @slow) and @integration")
    #[clap(long)]
    tags: Option<String>,
}

fn main() {
    let opts = Args::parse();

    for file in opts.files {
        let file_as_str = format!("{}", file.display());
        let src = std::fs::read_to_string(&file).unwrap();
        let (output, errors) = document_p().parse(&src).into_output_errors();

        if !errors.is_empty() {
            errors.into_iter().for_each(|e| {
                let src = src.clone();
                Report::build(
                    ReportKind::Error,
                    (file_as_str.clone(), e.span().into_range()),
                )
                .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                .with_message(e.to_string())
                .with_label(
                    Label::new((file_as_str.clone(), e.span().into_range()))
                        .with_message(e.reason().to_string())
                        .with_color(Color::Red),
                )
                .finish()
                .print(sources([(file_as_str.clone(), src)]))
                .unwrap()
            });
            return;
        } else {
            let document = output.expect("must always produce a document");

            let all_steps = document.steps();
            let missing: Vec<&cucumber_gherkin::gherkin::Step> = all_steps
                .iter()
                .filter(|step| <DefaultSteps as Steps>::get_step_fn(&DefaultSteps, step).is_none())
                .collect();

            for step in missing {
                Report::build(ReportKind::Error, (file_as_str.clone(), 0..1))
                    .with_message(format!(
                        "Missing step definition for '{}'",
                        step.text_as_string()
                    ))
                    .with_label(
                        Label::new((file_as_str.clone(), 0..1))
                            .with_message("Missing step definition")
                            .with_color(Color::Red),
                    )
                    .finish()
                    .print(sources([(file_as_str.clone(), src.clone())]))
                    .unwrap();
            }

            if opts.dry_run {
                return;
            }

            let runner = Runner;

            // Parse tag filter if provided
            if let Some(tag_expr_str) = &opts.tags {
                match TagOperation::parse(tag_expr_str) {
                    Ok(tag_expr) => {
                        println!("🏷️ Running with tag filter: {}", tag_expr_str);
                        let filter = RunnerFilter::with_tags(tag_expr);
                        runner.run_with_filter(&document, DefaultSteps, &filter);
                    }
                    Err(e) => {
                        eprintln!("❌ Invalid tag expression '{}': {}", tag_expr_str, e);
                        std::process::exit(1);
                    }
                }
            } else {
                runner.run(&document, DefaultSteps);
            }
        }
    }
}
