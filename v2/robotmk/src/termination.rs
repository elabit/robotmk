use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct TerminationFlag(Arc<AtomicBool>);

impl TerminationFlag {
    pub fn new(raw_flag: Arc<AtomicBool>) -> Self {
        Self(raw_flag)
    }

    pub fn should_terminate(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
