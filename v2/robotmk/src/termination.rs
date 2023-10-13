use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn start_termination_control() -> Result<TerminationFlag> {
    let raw_flag = Arc::new(AtomicBool::new(false));
    let raw_flag_clone = raw_flag.clone();
    ctrlc::set_handler(move || {
        raw_flag_clone.store(true, Ordering::Relaxed);
    })
    .context("Failed to register signal handler for CTRL+C")?;
    Ok(TerminationFlag(raw_flag))
}

#[derive(Clone)]
pub struct TerminationFlag(Arc<AtomicBool>);

impl TerminationFlag {
    pub fn should_terminate(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl TerminationFlag {
        pub fn new() -> Self {
            Self(Arc::new(AtomicBool::new(false)))
        }
    }
}
