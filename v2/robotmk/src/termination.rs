use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::{task::yield_now, time::sleep};

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

pub enum Outcome<T> {
    Cancel,
    Timeout,
    Completed(T),
}

#[tokio::main]
pub async fn waited<F>(duration: Duration, flag: &TerminationFlag, future: F) -> Outcome<F::Output>
where
    F: Future,
{
    async fn cancelled(flag: &TerminationFlag) {
        while !flag.should_terminate() {
            yield_now().await
        }
    }

    tokio::select! {
        outcome = future => { Outcome::Completed(outcome) },
        _ = cancelled(flag) => { Outcome::Cancel },
        _ = sleep(duration) => { Outcome::Timeout },
    }
}
