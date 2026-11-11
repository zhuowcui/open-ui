use std::time::Duration;

use openui::prelude::*;
use openui::task::{spawn, spawn_local, time};

fn clock_and_work() -> ViewNode {
    let seconds = create_signal(0u64);
    let primes = create_signal(None::<usize>);

    // On the UI thread: may hold signals and elements.
    spawn_local(async move {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            seconds.update(|s| *s += 1);
        }
    })
    .detach();

    // Heavy work on a background thread, then back on the UI thread.
    spawn_local(async move {
        let count = spawn(async move { count_primes(200_000) }).await;
        primes.set(Some(count));
    })
    .detach();

    view! {
        <div>
            <p>{format!("Running for {} s", seconds.get())}</p>
            <p>{match primes.get() {
                Some(count) => format!("{count} primes below 200,000"),
                None => "Counting primes…".to_string(),
            }}</p>
        </div>
    }
}

fn count_primes(limit: usize) -> usize {
    (2..limit)
        .filter(|&n| (2..n).take_while(|d| d * d <= n).all(|d| n % d != 0))
        .count()
}
