use crate::config::config::{Binding, Point};

#[cfg_attr(test, mockall::automock)]
pub trait Recognizer: Send {
    fn find_candidate(&self, points: &[Point], bindings: &Vec<Binding>) -> Option<Candidate>;
}

#[derive(Clone)]
pub struct Candidate {
    pub binding: Binding,
    pub diff: f64,
}
