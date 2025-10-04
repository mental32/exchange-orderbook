use crate::gherkin::Step;

#[derive(Debug)]
pub enum StepOutput {
    Passed,
    Failed,
    Skipped,
}

pub type StepFn = Box<dyn Fn(&Step) -> StepOutput + 'static>;

pub trait Steps: Send + Sync {
    fn get_step_fn(&self, step: &Step) -> Option<StepFn>;
}

impl Steps for () {
    fn get_step_fn(&self, _step: &Step) -> Option<StepFn> {
        None
    }
}

#[derive(Debug)]
pub struct DefaultSteps;

impl Steps for DefaultSteps {
    fn get_step_fn(&self, step: &Step) -> Option<StepFn> {
        None
    }
}
