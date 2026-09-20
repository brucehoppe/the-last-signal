//! Talks to a real local Ollama model. Not run in CI:
//! cargo test --release --test real_model -- --ignored --nocapture
#![cfg(not(target_arch = "wasm32"))]
use the_last_signal::{ai, core::Game};

#[test]
#[ignore]
fn echo_reads_a_damaged_record_and_answers_the_terminal_from_it() {
    let config = ai::Config::load().expect("config.json");
    let mut g = Game::new(4);
    g.enemies.clear();
    g.seen.fill(true);
    let blind = ai::request(
        &config,
        ai::payload(&g, "Can we answer the terminal yet?", &config),
    )
    .expect("model reply");
    println!("no records: {blind}");
    assert!(
        !blind.contains("North Station"),
        "the model must not know an unrecovered record"
    );
    g.player = g.archives[1].pos;
    g.interact();
    for q in [
        "What does the damaged record say?",
        "Can we answer the terminal yet?",
    ] {
        let t = std::time::Instant::now();
        let reply = ai::request(&config, ai::payload(&g, q, &config)).expect("model reply");
        println!("{q}\n  -> {reply}\n  ({:.1}s)", t.elapsed().as_secs_f32());
        assert!(reply.contains("North Station"), "{reply}");
    }
}
