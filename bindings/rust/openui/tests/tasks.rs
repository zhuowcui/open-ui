//! UI and background tasks, signal senders and resources, driven headless
//! by `HeadlessApp::run_until_idle`.
#![cfg(feature = "async")]

use openui::task::{spawn, spawn_blocking, spawn_local, time};
use openui::{
    create_resource, create_scope, create_signal, dispose_scope, HeadlessApp, ViewportMetrics,
};
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

fn app() -> HeadlessApp {
    HeadlessApp::new(ViewportMetrics::from_logical_size(200.0, 100.0, 1.0).unwrap()).unwrap()
}

#[test]
fn ui_tasks_await_background_work_and_timers_then_set_signals() {
    let mut app = app();
    let total = create_signal(0_u64);
    let text = create_signal(String::new());
    let started = Instant::now();
    spawn_local(async move {
        let sum = spawn(async { (1..=1000_u64).sum::<u64>() }).await;
        time::sleep(Duration::from_millis(20)).await;
        let joined = spawn_blocking(|| ["a", "b"].join("+")).await;
        total.set(sum);
        text.set(joined);
    })
    .detach();
    // Nothing runs until the UI thread runs its tasks.
    assert_eq!(total.get(), 0);
    app.run_until_idle().unwrap();
    assert_eq!(total.get(), 500_500);
    assert_eq!(text.get(), "a+b");
    assert!(started.elapsed() >= Duration::from_millis(20));
}

#[test]
fn intervals_tick_repeatedly() {
    let mut app = app();
    let ticks = create_signal(0);
    spawn_local(async move {
        let mut interval = time::interval(Duration::from_millis(5));
        for _ in 0..3 {
            interval.tick().await;
            ticks.update(|count| *count += 1);
        }
    })
    .detach();
    app.run_until_idle().unwrap();
    assert_eq!(ticks.get(), 3);
}

#[test]
fn disposing_a_scope_cancels_its_tasks_and_dropping_a_task_cancels_it() {
    let mut app = app();
    let finished = Rc::new(Cell::new(0));
    let dropped = Rc::new(Cell::new(0));
    struct OnDrop(Rc<Cell<u32>>);
    impl Drop for OnDrop {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    let scope = create_scope(|| {
        let finished = finished.clone();
        let guard = OnDrop(dropped.clone());
        spawn_local(async move {
            let _guard = guard;
            time::sleep(Duration::from_millis(30)).await;
            finished.set(finished.get() + 1);
        })
        .detach();
    });
    let task = {
        let finished = finished.clone();
        let guard = OnDrop(dropped.clone());
        spawn_local(async move {
            let _guard = guard;
            time::sleep(Duration::from_millis(30)).await;
            finished.set(finished.get() + 1);
        })
    };
    // Let both start and reach their timers.
    app.run_ready_tasks().unwrap();
    dispose_scope(scope);
    drop(task);
    app.run_until_idle().unwrap();
    assert_eq!(finished.get(), 0, "neither task may finish");
    assert_eq!(dropped.get(), 2, "both futures are dropped");
}

#[test]
fn senders_set_signals_from_other_threads_and_ignore_disposed_signals() {
    let mut app = app();
    let value = create_signal(0);
    let sender = value.sender();
    std::thread::spawn(move || sender.send(42)).join().unwrap();
    assert_eq!(value.get(), 0, "applied on the UI thread only");
    app.run_until_idle().unwrap();
    assert_eq!(value.get(), 42);

    let scope = create_scope(|| {
        let short_lived = create_signal(0);
        let sender = short_lived.sender();
        std::thread::spawn(move || sender.send(1)).join().unwrap();
    });
    dispose_scope(scope);
    app.run_until_idle().unwrap();
}

#[test]
fn resources_follow_their_source_and_keep_only_the_latest_result() {
    let mut app = app();
    let source = create_signal(1_u32);
    let fetches = Rc::new(Cell::new(0));
    let counter = fetches.clone();
    let resource = create_resource(
        move || source.get(),
        move |input| {
            counter.set(counter.get() + 1);
            async move {
                // Later inputs finish sooner, so a stale fetch would land last.
                time::sleep(Duration::from_millis(40 / u64::from(input))).await;
                if input == 0 {
                    Err("zero".to_owned())
                } else {
                    Ok(input * 10)
                }
            }
        },
    );
    assert!(resource.loading());
    assert_eq!(resource.get(), None);
    app.run_until_idle().unwrap();
    assert!(!resource.loading());
    assert_eq!(resource.get(), Some(Ok(10)));

    source.set(2);
    source.set(4);
    assert!(resource.loading());
    assert_eq!(
        resource.get(),
        Some(Ok(10)),
        "the previous result stays until the next"
    );
    app.run_until_idle().unwrap();
    assert_eq!(resource.get(), Some(Ok(40)));

    resource.refetch();
    app.run_until_idle().unwrap();
    assert_eq!(fetches.get(), 4);
    assert_eq!(resource.get(), Some(Ok(40)));
}
