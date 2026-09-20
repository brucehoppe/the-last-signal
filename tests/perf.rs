//! Run with: cargo test --release --test perf -- --ignored --nocapture
use std::time::Instant;
use the_last_signal::core::Game;

fn time<F: FnMut()>(label: &str, iters: u32, mut f: F) {
    let t = Instant::now();
    for _ in 0..iters {
        f();
    }
    let per = t.elapsed().as_secs_f64() * 1e6 / iters as f64;
    println!("{label:<34} {per:>9.2} us/iter");
}

#[test]
#[ignore]
fn core_hot_paths() {
    let mut seed = 0;
    time("Game::new", 2000, || {
        seed += 1;
        std::hint::black_box(Game::new(seed));
    });
    let mut g = Game::new(42);
    time("update_visibility(7)", 20000, || g.update_visibility(7));
    time("update_visibility(11)", 20000, || g.update_visibility(11));
    g.seen.fill(true);
    time("knowledge() (all discovered)", 5000, || {
        std::hint::black_box(g.knowledge());
    });
    let relay = g.relay;
    time("known_route(relay)", 5000, || {
        std::hint::black_box(g.known_route(relay));
    });
    for _ in 0..2000 {
        g.log("bench", "event text of a realistic length for the log");
    }
    time("log() at the 2000-event cap", 20000, || {
        g.log("bench", "event text of a realistic length for the log")
    });
}
