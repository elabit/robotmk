use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

pub enum Outcome<T> {
    Cancel,
    Timeout,
    Completed(T),
}

#[tokio::main]
pub async fn waited<F>(
    duration: Duration,
    flag: &CancellationToken,
    future: F,
) -> Outcome<F::Output>
where
    F: Future,
{
    tokio::select! {
        outcome = future => { Outcome::Completed(outcome) },
        _ = flag.cancelled() => { Outcome::Cancel },
        _ = sleep(duration) => { Outcome::Timeout },
    }
}
